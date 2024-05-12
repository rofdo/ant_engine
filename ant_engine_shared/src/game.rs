
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

#[derive(Debug, PartialEq)]
pub enum Command {
    Add(i32),
    Stop,
}

#[derive(Debug, Clone, PartialEq)]
pub struct State {
    test_value: i32,
}

impl State {
    pub fn new() -> State {
        State { test_value: 0 }
    }

    pub fn add_value(&mut self, value: i32) {
        self.test_value += value;
    }
}

pub struct Game {
    running: bool,
    state: State,
    step: usize,
    command_channel: Receiver<Command>,
    state_channel: Sender<State>,
}

impl Game {
    fn receive_commands(&self) -> Vec<Command> {
        let mut commands = vec![];
        while let Ok(command) = self.command_channel.try_recv() {
            commands.push(command);
        }
        commands
    }

    fn step(&mut self, commands: Vec<Command>) {
        for command in commands {
            match command {
                Command::Add(value) => self.state.add_value(value),
                Command::Stop => {
                    println!("Game stopped at step {}", self.step);
                    self.running = false;
                    return;
                }
            }
        }
        self.step += 1;
    }

    fn send_state(&self) {
        self.state_channel.send(self.state.clone()).unwrap();
    }

    fn run(&mut self) {
        while self.running {
            let commands = self.receive_commands();
            self.step(commands);
            self.send_state();
        }
    }
}

pub struct GameHandler {
    thread: JoinHandle<()>,
    command_channel: Sender<Command>,
    state_channel: Receiver<State>,
}

impl GameHandler {
    pub fn send(&self, command: Command) -> Result<(), Box<dyn std::error::Error>> {
        self.command_channel.send(command).map_err(|_| "Failed to send command".into())
    }

    pub fn stop(self) -> Result<(), Box<dyn std::error::Error>> {
        self.send(Command::Stop)?;
        self.thread.join().unwrap();
        Ok(())
    }

    pub fn last_state(&self) -> Result<State, Box<dyn std::error::Error>> {
        self.state_channel.recv().map_err(|_| "Failed to receive state".into())
    }
}

impl Game {
    fn new(command_rx: mpsc::Receiver<Command>, state_tx: mpsc::Sender<State>) -> Game {
        Game {
            running: true,
            state: State::new(),
            step: 0,
            command_channel: command_rx,
            state_channel: state_tx,
        }
    }

    pub fn start() -> GameHandler {
        let (command_tx, command_rx) = mpsc::channel();
        let (state_tx, state_rx) = mpsc::channel();
        let mut game = Game::new(command_rx, state_tx);

        let thread = thread::spawn(move || {
            game.run();
        });
        GameHandler {
            thread,
            command_channel: command_tx,
            state_channel: state_rx,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receive_commands() {
        let (command_tx, command_rx) = mpsc::channel();
        let (state_tx, _) = mpsc::channel();

        let game = Game::new(command_rx, state_tx);
        command_tx.send(Command::Add(2)).unwrap();

        let commands = game.receive_commands();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0], Command::Add(2));
    }

    #[test]
    fn start_stop_game() {
        let game_handler = Game::start();
        game_handler.send(Command::Add(2)).unwrap();
        game_handler.send(Command::Add(3)).unwrap();
        game_handler.stop().unwrap();
    }

    #[test]
    fn receive_state() {
        let game_handler = Game::start();
        game_handler.send(Command::Add(2)).unwrap();
        let state = game_handler.last_state().unwrap();
        assert_eq!(state, State { test_value: 2 });
        game_handler.stop().unwrap();
    }
}
