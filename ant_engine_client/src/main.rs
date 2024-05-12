use ant_engine_shared::game::{Game, Command};

fn main() {
    let game_handler = Game::start();
    // listen for keepresses
    loop {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        let trimmed = input.trim();
        match trimmed {
            "stop" => {
                game_handler.stop().unwrap();
                break;
            }
            "view" => {
                let state = game_handler.last_state().unwrap();
                println!("State: {:?}", state);
            }
            _ => {
                let num: i32 = match trimmed.parse() {
                    Ok(num) => num,
                    Err(_) => {
                        println!("Please enter a number or 'stop'");
                        continue;
                    }
                };
                game_handler.send(Command::Add(num)).unwrap();
            }
        }
    }
}
