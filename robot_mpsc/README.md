# Robot Assembly MPSC

## Challenge Description

Simulate a robot assembly factory with multiple concurrent producers creating robot parts (skeletons and motors) and a consumer that assembles complete robots from these parts. The system must:

- Run multiple producers concurrently, each creating parts at different rates
- Use bounded channels to manage production flow and prevent unbounded memory growth
- Implement graceful shutdown on Ctrl+C signal
- Synchronize parts from different producers using a fan-in pattern
- Load configuration from YAML file for production rates and buffer capacities

## Solution

This solution uses:
- **Crossbeam** for scoped threads to manage producer/consumer lifecycle
- **Crossbeam-channel** for bounded MPSC (multi-producer, single-consumer) channels
- **Atomic state management** for coordinated shutdown across threads
- **Serde & serde_yaml** for configuration management
- **ctrlc** crate for signal handling

### Key Features:
- Three producer threads:
  - Skeleton producer (500ms delay, 10 capacity)
  - Motor producer (1000ms delay, 5 capacity)
  - Robot assembler (2000ms delay, 3 capacity)
- Fan-in pattern to zip skeleton and motor streams into robots
- Graceful shutdown with three states: Running → ShuttingDown → Terminated
- Configurable production rates via YAML
- Buffered assembly using VecDeque for part synchronization

### Architecture:
```
[Skeleton Producer] ─→ skeleton_channel ─┐
                                          ├─→ [Robot Producer] ─→ robot_channel ─→ [Main Consumer]
[Motor Producer]    ─→ motor_channel    ─┘
```

### Shutdown Behavior:
1. Ctrl+C triggers transition from Running to ShuttingDown
2. Producers stop creating new parts
3. Robot producer drains remaining buffered parts
4. State transitions to Terminated
5. Main thread finishes processing remaining robots and exits

## Steps to Run

### Prerequisites
- Rust (2024 edition or later)

### Running the Challenge

1. Navigate to the challenge directory:
   ```bash
   cd robot_mpsc
   ```

2. Review the configuration (optional):
   ```bash
   cat config/config.yaml
   ```

3. Run the program:
   ```bash
   cargo run
   ```

4. Observe the output:
   ```
   Assembled robot: Robot { id: 0, skeleton: { id: 0, hardness: 45 }, motor: { id: 0, rpm: 523 } }
   Assembled robot: Robot { id: 1, skeleton: { id: 1, hardness: 78 }, motor: { id: 1, rpm: 892 } }
   ...
   ```

5. Press **Ctrl+C** to trigger graceful shutdown:
   ```
   ^CShutting down...
   Shutdown completed
   ```

### Build for Release

For better performance:
```bash
cargo build --release
./target/release/robot_mpsc
```

### Customize Production Rates

Edit `config/config.yaml` to adjust delays and buffer capacities:
```yaml
assembling_rates:
  skeleton_producer:
    delay: 500    # milliseconds per skeleton
    capacity: 10  # max skeletons in buffer
  motor_producer:
    delay: 1000   # milliseconds per motor
    capacity: 5   # max motors in buffer
  robot_producer:
    delay: 2000   # milliseconds per robot
    capacity: 3   # max robots in buffer
```

## Output

The program continuously prints assembled robots with:
- Robot ID (synchronized with skeleton ID)
- Skeleton properties (ID and random hardness 0-100)
- Motor properties (ID and random RPM 0-1000)

The simulation runs until manually stopped with Ctrl+C, at which point it gracefully shuts down and processes remaining buffered parts.

