use log::warn;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct State {
    test_value: i32,
}

impl State {
    pub fn new() -> State {
        State { test_value: 0 }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum GameCommand {
    TestAdd(i32),
    TestMul(i32),
    Quit,
}

#[derive(Debug)]
pub struct Game {
    running: bool,
    state: State,
    command_receiver: Receiver<GameCommand>,
    state_sender: Sender<State>,
}

impl Game {
    fn read_commands(&self) -> Vec<GameCommand> {
        let mut commands = Vec::new();
        while let Ok(command) = self.command_receiver.try_recv() {
            commands.push(command);
        }
        commands
    }

    fn step(&mut self, commands: Vec<GameCommand>) {
        for command in commands {
            match command {
                GameCommand::TestAdd(value) => self.state.test_value += value,
                GameCommand::TestMul(value) => self.state.test_value *= value,
                GameCommand::Quit => self.running = false,
            }
        }
    }

    fn send_state(&self) {
        let result = self.state_sender.send(self.state.clone());
        match result {
            Ok(_) => (),
            Err(e) => warn!("Error sending state: {}", e),
        }
    }

    pub fn run(&mut self) {
        self.running = true;
        while self.running {
            let commands = self.read_commands();
            self.step(commands);
            self.send_state();
        }
    }
}

#[derive(Debug)]
pub struct GameHandle {
    command_sender: Sender<GameCommand>,
    state_receiver: Receiver<State>,
    thread_handle: JoinHandle<()>,
}

impl GameHandle {
    pub fn send_command(&self, command: GameCommand) -> Result<(), mpsc::SendError<GameCommand>> {
        self.command_sender.send(command)
    }

    pub fn receive_state(&self) -> Result<State, mpsc::RecvError> {
        self.state_receiver.recv()
    }

    pub fn stop(self) -> Result<(), mpsc::SendError<GameCommand>> {
        self.command_sender.send(GameCommand::Quit)?;
        self.thread_handle
            .join()
            .unwrap_or_else(|_| warn!("Error joining thread"));
        Ok(())
    }
}

// unit tests
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_game() {
        let (command_sender, command_receiver) = mpsc::channel();
        let (state_sender, state_receiver) = mpsc::channel();
        let mut game = Game {
            running: false,
            state: State::new(),
            command_receiver,
            state_sender,
        };
        let thread_handle = thread::spawn(move || game.run());
        let game_handle = GameHandle {
            command_sender,
            state_receiver,
            thread_handle,
        };
        game_handle.send_command(GameCommand::TestAdd(1)).unwrap();
        game_handle.send_command(GameCommand::TestMul(2)).unwrap();
        let state = game_handle.receive_state().unwrap();
        assert_eq!(state.test_value, 2);
        game_handle
            .stop()
            .unwrap_or_else(|e| panic!("Error stopping game: {}", e));
    }
}
