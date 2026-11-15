use std::cell::RefCell;
use std::collections::VecDeque;
use std::io;

use super::{Device, run};
use crate::log::LogBuffer;
use crate::session::Session;
use crate::test_support::{TempDir, fixed_clock, init_repo, test_git};
use ui::layout::Size;

const SMALL: Size = Size {
    width: 80,
    height: 24,
};

const LARGE: Size = Size {
    width: 100,
    height: 30,
};

/// A terminal that answers sizes and input from queues and records the frames. When the
/// sizes run out, the size stays at the last one; when the input runs out, reading fails,
/// so that a test whose session never quits ends.
struct FakeDevice {
    sizes: RefCell<VecDeque<io::Result<Size>>>,
    last_size: RefCell<Size>,
    input: RefCell<VecDeque<Vec<u8>>>,
    frames: RefCell<Vec<String>>,
    draw_fails: bool,
}

impl FakeDevice {
    fn new(sizes: Vec<io::Result<Size>>, input: &[&[u8]]) -> Self {
        FakeDevice {
            sizes: RefCell::new(sizes.into_iter().collect()),
            last_size: RefCell::new(SMALL),
            input: RefCell::new(input.iter().map(|bytes: &&[u8]| bytes.to_vec()).collect()),
            frames: RefCell::new(Vec::new()),
            draw_fails: false,
        }
    }
}

impl Device for FakeDevice {
    fn size(&self) -> io::Result<Size> {
        let next: Option<io::Result<Size>> = self.sizes.borrow_mut().pop_front();
        let size: Size = next.unwrap_or(Ok(*self.last_size.borrow()))?;
        *self.last_size.borrow_mut() = size;
        Ok(size)
    }

    fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
        let bytes: Vec<u8> = self
            .input
            .borrow_mut()
            .pop_front()
            .ok_or_else(|| io::Error::other("no more input"))?;
        buffer[..bytes.len()].copy_from_slice(&bytes);
        Ok(bytes.len())
    }

    fn draw(&self, frame: &str) -> io::Result<()> {
        if self.draw_fails {
            return Err(io::Error::other("cannot draw"));
        }
        self.frames.borrow_mut().push(frame.to_string());
        Ok(())
    }
}

fn session(dir: &TempDir) -> Session {
    init_repo(dir.path());
    Session::new(
        test_git(dir.path()),
        LogBuffer::new(fixed_clock),
        Box::new(|_: &std::path::Path| Ok(())),
        Box::new(|run: &mut dyn FnMut()| {
            run();
            Ok(())
        }),
    )
}

#[test]
fn draws_at_the_start_and_after_input_but_not_when_idle() {
    let dir: TempDir = TempDir::new();
    let mut session: Session = session(&dir);
    let device: FakeDevice = FakeDevice::new(vec![], &[b"", b"j", b"", b"qy"]);
    run(&device, &mut session).unwrap();
    assert_eq!(device.frames.borrow().len(), 2);
    assert!(session.should_quit());
}

#[test]
fn redraws_when_the_size_changes() {
    let dir: TempDir = TempDir::new();
    let mut session: Session = session(&dir);
    let device: FakeDevice = FakeDevice::new(vec![Ok(SMALL), Ok(LARGE)], &[b"", b"qy"]);
    run(&device, &mut session).unwrap();
    assert_eq!(device.frames.borrow().len(), 2);
    assert_eq!(*device.last_size.borrow(), LARGE);
}

#[test]
fn device_failures_end_the_loop() {
    let dir: TempDir = TempDir::new();
    let mut session: Session = session(&dir);
    let no_size: FakeDevice = FakeDevice::new(vec![Err(io::Error::other("no size"))], &[]);
    let size_error: io::Error = run(&no_size, &mut session).unwrap_err();
    assert_eq!(size_error.to_string(), "no size");
    let no_input: FakeDevice = FakeDevice::new(vec![], &[]);
    let input_error: io::Error = run(&no_input, &mut session).unwrap_err();
    assert_eq!(input_error.to_string(), "no more input");
    let no_drawing: FakeDevice = FakeDevice {
        draw_fails: true,
        ..FakeDevice::new(vec![], &[])
    };
    let draw_error: io::Error = run(&no_drawing, &mut session).unwrap_err();
    assert_eq!(draw_error.to_string(), "cannot draw");
}
