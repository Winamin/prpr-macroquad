use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

// ========== FrameFuture next_frame().await) ==========
#[derive(Default)]
pub struct FrameFuture {
    done: bool,
}

impl Future for FrameFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Self::Output> {
        if self.done {
            Poll::Ready(())
        } else {
            self.done = true;
            Poll::Pending
        }
    }
}

// ========== FileLoadingFuture ==========
type FileResult<T> = Result<T, std::io::Error>;

pub struct FileLoadingFuture {
    contents: Arc<Mutex<Option<FileResult<Vec<u8>>>>,
}

impl FileLoadingFuture {
    pub fn new(path: String) -> Self {
        let contents = Arc::new(Mutex::new(None));
        let contents_clone = contents.clone();

        std::thread::spawn(move || {
            let data = std::fs::read(path);
            *contents_clone.lock().unwrap() = Some(data);
        });

        Self { contents }
    }
}

impl Future for FileLoadingFuture {
    type Output = FileResult<Vec<u8>>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Self::Output> {
        if let Some(result) = self.contents.lock().unwrap().take() {
            Poll::Ready(result)
        } else {
            Poll::Pending
        }
    }
}

// ========== Waker ==========
fn waker() -> Waker {
    unsafe fn clone(_data: *const ()) -> RawWaker {
        RawWaker::new(std::ptr::null(), &VTABLE)
    }
    unsafe fn wake(_data: *const ()) {
    }
    unsafe fn wake_by_ref(_data: *const ()) {
    }
    unsafe fn drop(_data: *const ()) {
    }
    const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);
    let raw_waker = RawWaker::new(std::ptr::null(), &VTABLE);
    unsafe { Waker::from_raw(raw_waker) }
}

pub fn resume<T>(future: &mut Pin<Box<dyn Future<Output = T>>>) -> Option<T> {
    let waker = waker();
    let mut cx = Context::from_waker(&waker);
    match future.as_mut().poll(&mut cx) {
        Poll::Ready(v) => Some(v),
        Poll::Pending => None,
    }
}

#[macroquad::main("Future Test")]
async fn main() {
    let mut frame_future = Box::pin(FrameFuture::default());
    
    let mut file_future = Box::pin(FileLoadingFuture::new("Cargo.toml".to_string()));

    loop {
        if let Some(()) = resume(&mut frame_future) {
            println!("FrameFuture completed!");
            frame_future = Box::pin(FrameFuture::default());
        }

        if let Some(result) = resume(&mut file_future) {
            match result {
                Ok(data) => println!("Loaded file ({} bytes)", data.len()),
                Err(e) => println!("Error loading file: {:?}", e),
            }
            break;
        }

        clear_background(WHITE);
        next_frame().await;
    }
}