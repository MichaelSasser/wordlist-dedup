use std::{
    io::{
        stdout,
        Write,
    },
    sync::mpsc::{
        channel,
        Sender,
        TryRecvError,
    },
    thread::{
        self,
        JoinHandle,
    },
    time::{
        Duration,
        Instant,
    },
};

pub struct Spinner {
    sender: Sender<Instant>,
    time_started: Instant,
    join: Option<JoinHandle<()>>,
}

// Ensure the spinner shuts down gracefully
impl Drop for Spinner {
    fn drop(&mut self) {
        self.finish(None).expect("Unable to wrap up the spinner.");
    }
}

impl Spinner {
    pub fn new(spinner: Vec<&'static str>, message: String) -> Self {
        let time_started = Instant::now();
        let (sender, recv) = channel::<Instant>();

        let join = thread::spawn(move || {
            let stream = stdout();
            'outer: loop {
                for frame in spinner.iter() {
                    let (on_stop, time) = match recv.try_recv() {
                        Ok(time) => (true, time),
                        Err(TryRecvError::Disconnected) => {
                            (true, Instant::now())
                        },
                        Err(TryRecvError::Empty) => (false, Instant::now()),
                    };

                    let duration =
                        time.duration_since(time_started).as_secs_f64();
                    {
                        let mut stream = stream.lock();
                        write!(
                            stream,
                            "\x1b[2K\r  \x1b[36m{}\x1b[0m {} ({:.1} s)",
                            frame, message, duration
                        )
                        .expect("Unable to write to stdout.");
                        stream.flush().expect("Unable to flush stream.");
                    }

                    if on_stop {
                        break 'outer;
                    }

                    thread::sleep(Duration::from_millis(100));
                }
            }
        });

        Self {
            sender,
            time_started,
            join: Some(join),
        }
    }

    pub fn finish(
        &mut self,
        msg: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.join.is_some() {
            self.sender.send(Instant::now()).unwrap();
            self.join.take().unwrap().join().unwrap();
        }

        if let Some(msg) = msg {
            let time = Instant::now();
            let duration =
                time.duration_since(self.time_started).as_secs_f64();

            let mut stream = stdout().lock();
            writeln!(stream, "\x1b[2K\r  {} in {:.1} s.", msg, duration)?;
            stream.flush()?;
        }
        Ok(())
    }
}
