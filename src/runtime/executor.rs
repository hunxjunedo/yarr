use crate::future::{
    Future,
    PollState::{NotReady, Ready},
};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::{self, Thread},
};

#[derive(Clone)]
pub struct Waker {
    //something responsible for waking a particular thread
    thread: Thread, //handle to the thread which is parked, waiting for some event
    id: usize,
    ready_queue: Arc<Mutex<Vec<usize>>>, //remember, a thread might be parked but could be becuase of more than one events. Each waker is associated with an event/task. While Waker does unpark a thread, the thread still has to check which event triggered, since other events might still be uncomplete. Therefore push our id to a vec, which is mutable across threads (done so by mutex) and owned by many (done so by arc).
}

impl Waker {
    pub fn wake(&self) {
        //push to the list to let the thread know this task can progress
        self.ready_queue
            .lock()
            .map(|mut q| q.push(self.id))
            .unwrap();
        //now unpark
        self.thread.unpark();
    }
}

type Task = Box<dyn Future<Output = String>>;
#[derive(Default)]
struct ExecutorCore {
    //remember, since this is a static variable, you cannot mutate, you need interior mutability
    tasks: RefCell<HashMap<usize, Task>>,
    ready_queue: Arc<Mutex<Vec<usize>>>, // stores the same thing as in the waker struct, clever way to share data
    next_id: Cell<usize>,
}

thread_local! {
    //stores program wide data, accessible from anywhere. lives as long as the process is.
    static CURRENT_EXEC: ExecutorCore = ExecutorCore::default();
}

//spawning new futures. aka handling them to the executor
pub fn spawn<F>(f: F)
where
    F: Future<Output = String> + 'static,
{
    //register interest.
    CURRENT_EXEC.with(|v| {
        let id = v.next_id.get();
        v.tasks.borrow_mut().insert(id, Box::new(f));
        //we have told the executor that this is also one of the tasks it should be handling
        //now tell it to poll it also, since futures are eager, and you have to poll first time also
        v.next_id.set(id + 1);
        v.ready_queue.lock().unwrap().push(id);
    });
}

//this should not be our public API.
//lets introduce one
pub struct Executor;
//no need for values, since that is already internally persistent

impl Executor {
    pub fn new() -> Self {
        Self {}
    }

    //id means the unique top-level task. when our I/O calls regsiter interest, they pass a waker which has embedded into it an id associated with that particular event. many wakers might wake up the thread, but every waker is able to tell who is responsible for triggering wake-up/unpark
    //this id allows the executor to decide which top-level future is now ready to poll
    //this waker can be passed on when registering interest with another api, and that api will use the
    pub fn get_waker(&self, id: usize) -> Waker {
        Waker {
            thread: thread::current(),
            id,
            ready_queue: CURRENT_EXEC.with(|v| Arc::clone(&v.ready_queue)),
        }
    }

    //what happens when executor actually wants to poll the futures present in the ready queue ? Those are waiting to be progressed.
    //Suppose we had a multi-threaded environment where work-stealing was also possible. If an executor (which there will be many of, running on multiple threads) wants to poll some future till waitpoint, it has to remove it from the list so others can't also poll it.
    //therefore, remove future from the list, poll it, if you get NotReady put it back. if you get ready, well you are done!

    fn get_future(&self, id: usize) -> Option<Task> {
        CURRENT_EXEC.with(|v| v.tasks.borrow_mut().remove(&id))
    }

    //part 2: once the executor has done polling a future, and recieved NotReady. which means it can now leave it for a while, add it back to the tasks. This is the opposite of get_future.
    //if it recieved ready, it would mean no more progress to make, and no need to add it back
    fn new_task(&self, id: usize, task: Task) {
        CURRENT_EXEC.with(|v| v.tasks.borrow_mut().insert(id, task));
    }

    //now the part where we actually pick up the the futures that are ready to poll, and poll them one by one.
    //this will be in a continous loop, if no one is ready we just park the thread! wait for someone to be ready, which will unpark! How elegant!
    //we take future that's ready to be polled (aka it caused a wake to be called). we poll it. handle the business logic (NotReady vs Ready), and then pick another future.
    //using LIFO approach, so one that completed latest gets handled earliest
    fn get_ready_future(&self) -> Option<usize> {
        CURRENT_EXEC.with(|v| v.ready_queue.lock().unwrap().pop())
    }

    //one last thing: what if we have already processed all the top-level futures and no more tasks left ?
    //well just exit the executor, because main thread of application is blocking on us. otherwise the process will never end!
    fn no_tasks_left(&self) -> bool {
        CURRENT_EXEC.with(|v| v.tasks.borrow().len() == 0)
    }

    //finally, we want to provide a function that the main thread blocks on, until no tasks left.
    pub fn block_on<F>(&mut self, future: F)
    where
        F: Future<Output = String> + 'static,
    {
        //if we proceed with the following code, it will store the future in a box, which gives it a stable location, and no matter how much you move it around,
        //the future it is pointing to, will never move, and hence the internal pointers will never become invalid
        //what if we dont let it reach all that and directly move it and see it break ?
        let mut future = future;
        match future.poll(&self.get_waker(999)) {
            Ready(txt) => {
                //this whole optimization, which is return immideatily if it is ready immideatily, 
                //is just an excuse to prevent boxing, and break the whole thing
                return;
            },
            NotReady => {}
        }
        println!("Your future did not return Ready on the first poll. \n It has been polled once and we are going to move it, which means the internal pointers will become invalid, and the second poll will panic.");
        println!("If it had returned Ready on the first poll, there would have been no probem at all");
        //1. add to the tasks
        spawn(future);

        //2. start the main loop
        loop {
            //get futures one by one
            while let Some(id) = self.get_ready_future() {
                //get the actual future
                let ft = self.get_future(id);
                if ft.is_none() {
                    //this means we got woken by an event, associated with a top-level future that does not exist!
                    //it is not in the tasks list! so why did we woke up for this ?
                    //false wakeup, handle here
                    continue;
                }
                //          called shadowing
                let mut ft = ft.unwrap();
                match ft.poll(&self.get_waker(id)) {
                    NotReady => {
                        //add it back to the tasks, we have done polling it till waitpoint
                        self.new_task(id, ft);
                    }
                    Ready(_) => {
                        continue;
                    }
                }
            }
            //only come here if no more futures ready to poll
            //this DOES NOT MEAN that we are done, tasks are waiting on IO, they are not resolved yet. We can however sleep until any task wake us up
            if self.no_tasks_left() {
                //this means all tasks resolved
                break;
            } else {
                thread::park();
            }
        }
    }
}

//here is the main thing
