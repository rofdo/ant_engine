use std::thread::{self, JoinHandle};
use std::sync::mpsc::{self, Sender};

#[derive(Debug, PartialEq)]
pub enum Command {
    Add(i32),
    Stop,
}

pub struct Game {
    state: i32,
    step: usize,
    command_channel: mpsc::Receiver<Command>,
    running: bool,
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
                Command::Add(value) => self.state += value,
                Command::Stop => {
                    println!("Game stopped at step {}", self.step);
                    self.running = false;
                    return;
                }
            }
        }
        self.step += 1;
    }

    fn run(&mut self) {
        while self.running {
            let commands = self.receive_commands();
            self.step(commands);
        }
    }
}

pub struct GameHandler {
    thread: JoinHandle<()>,
    command_channel: Sender<Command>,
}

impl Game {
    fn new(rx: mpsc::Receiver<Command>) -> Game {
        Game {
            state: 0,
            step: 0,
            command_channel: rx,
            running: true,
        }
    }

    pub fn start() -> GameHandler {
        let (tx, rx) = mpsc::channel();
        let mut game = Game::new(rx);

        let thread = thread::spawn(move || {
            game.run();
        });
        GameHandler { thread , command_channel: tx }
    }
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    #[test]
    fn receive_commands() {
        let (tx, rx) = mpsc::channel();
        let game = Game::new(rx);
        tx.send(Command::Add(2)).unwrap();

        let commands = game.receive_commands();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0], Command::Add(2));
    }

    #[test]
    fn start_stop_game() {
        let game_handler = Game::start();
        game_handler.command_channel.send(Command::Add(2)).unwrap();
        game_handler.command_channel.send(Command::Add(3)).unwrap();
        game_handler.command_channel.send(Command::Stop).unwrap();
        game_handler.thread.join().unwrap();
    }
}
