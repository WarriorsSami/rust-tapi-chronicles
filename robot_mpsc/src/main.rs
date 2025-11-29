use crossbeam::scope;
use crossbeam_channel::{Receiver, Sender, bounded};
use rand::Rng;
use serde::Deserialize;
use std::collections::VecDeque;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct Config {
    assembling_rates: AssemblingRates,
}

#[derive(Debug, Deserialize)]
struct AssemblingRates {
    skeleton_producer: AssemblingRate,
    motor_producer: AssemblingRate,
    robot_producer: AssemblingRate,
}

#[derive(Debug, Deserialize)]
struct AssemblingRate {
    delay: u64,
    capacity: u64,
}

#[derive(Debug)]
struct Skeleton {
    id: u64,
    hardness: u8,
}

#[derive(Debug)]
struct Motor {
    id: u64,
    rpm: u16,
}

#[derive(Debug)]
struct Robot {
    id: u64,
    skeleton: Skeleton,
    motor: Motor,
}

impl std::fmt::Display for Robot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Robot {{ id: {}, skeleton: {{ id: {}, hardness: {} }}, motor: {{ id: {}, rpm: {} }} }}",
            self.id, self.skeleton.id, self.skeleton.hardness, self.motor.id, self.motor.rpm
        )
    }
}

#[repr(usize)]
enum State {
    Running = 0,
    ShuttingDown = 1,
    Terminated = 2,
}

fn main() -> anyhow::Result<()> {
    let config = load_config("config/config.yaml")?;
    let state = Arc::new(AtomicUsize::new(State::Running as usize));

    // producer channels
    let (skeleton_tx, skeleton_rx) =
        bounded::<Skeleton>(config.assembling_rates.skeleton_producer.capacity as usize);
    let (motor_tx, motor_rx) =
        bounded::<Motor>(config.assembling_rates.motor_producer.capacity as usize);

    // consumer channels
    let (robot_tx, robot_rx) =
        bounded::<Robot>(config.assembling_rates.robot_producer.capacity as usize);

    {
        let state_for_signal = Arc::clone(&state);
        ctrlc::set_handler(move || {
            let prev = state_for_signal.swap(State::ShuttingDown as usize, Ordering::SeqCst);
            if prev == State::Running as usize {
                eprintln!("Shutting down...");
            }
        })?;
    }

    scope(|s| {
        {
            let state = Arc::clone(&state);
            let tx = skeleton_tx;
            let asm_rate = config.assembling_rates.skeleton_producer;
            s.spawn(|_| launch_skeleton_producer(state, tx, asm_rate));
        }

        {
            let state = Arc::clone(&state);
            let tx = motor_tx;
            let asm_rate = config.assembling_rates.motor_producer;
            s.spawn(|_| launch_motor_producer(state, tx, asm_rate));
        }

        {
            let state = Arc::clone(&state);
            let s_rx = skeleton_rx.clone();
            let m_rx = motor_rx.clone();
            let tx = robot_tx.clone();
            let asm_rate = config.assembling_rates.robot_producer;
            s.spawn(|_| launch_robot_producer(state, s_rx, m_rx, tx, asm_rate));
        }

        for robot in robot_rx.iter() {
            println!("Assembled robot: {}", robot);

            if state.load(Ordering::Relaxed) == State::Terminated as usize {
                break;
            }
        }
    })
    .expect("Failed to launch scoped threads for producers");

    println!("Shutdown completed");

    Ok(())
}

fn load_config<P: AsRef<Path>>(path: P) -> anyhow::Result<Config> {
    let file = File::open(path)?;
    let cfg: Config = serde_yaml::from_reader(file)?;
    Ok(cfg)
}

fn launch_skeleton_producer(
    state: Arc<AtomicUsize>,
    tx: Sender<Skeleton>,
    asm_rate: AssemblingRate,
) {
    let mut id = 0_u64;
    let mut rng = rand::rng();

    while state.load(Ordering::Relaxed) == State::Running as usize {
        let skeleton = Skeleton {
            id,
            hardness: rng.random_range(0..=100),
        };

        thread::sleep(Duration::from_millis(asm_rate.delay));

        if tx.send(skeleton).is_err() {
            break;
        }

        id += 1;
    }
}

fn launch_motor_producer(state: Arc<AtomicUsize>, tx: Sender<Motor>, asm_rate: AssemblingRate) {
    let mut id = 0_u64;
    let mut rng = rand::rng();

    while state.load(Ordering::Relaxed) == State::Running as usize {
        let motor = Motor {
            id,
            rpm: rng.random_range(0..=1000),
        };

        thread::sleep(Duration::from_millis(asm_rate.delay));

        if tx.send(motor).is_err() {
            break;
        }

        id += 1;
    }
}

// leverage the zipped fan-in pattern
fn launch_robot_producer(
    state: Arc<AtomicUsize>,
    s_rx: Receiver<Skeleton>,
    m_rx: Receiver<Motor>,
    tx: Sender<Robot>,
    asm_rate: AssemblingRate,
) {
    // we don't need bounded buffers here since the channels already provide buffering
    let mut skeleton_buf = VecDeque::<Skeleton>::new();
    let mut motor_buf = VecDeque::<Motor>::new();

    let (mut s_open, mut m_open) = (true, true);

    while s_open || m_open || (!skeleton_buf.is_empty() && !motor_buf.is_empty()) {
        if s_open {
            match s_rx.recv() {
                Ok(skeleton) => skeleton_buf.push_back(skeleton),
                Err(_) => s_open = false,
            }
        }

        if m_open {
            match m_rx.recv() {
                Ok(motor) => motor_buf.push_back(motor),
                Err(_) => m_open = false,
            }
        }

        while let (Some(skeleton), Some(motor)) = (skeleton_buf.pop_front(), motor_buf.pop_front())
        {
            let robot = Robot {
                id: skeleton.id, // assuming skeleton and motor ids are synchronized
                skeleton,
                motor,
            };

            thread::sleep(Duration::from_millis(asm_rate.delay));

            if tx.send(robot).is_err() {
                (s_open, m_open) = (false, false);
                break;
            }
        }
    }

    // drain the remaining buffers
    state.store(State::Terminated as usize, Ordering::SeqCst);
}
