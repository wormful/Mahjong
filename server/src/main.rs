extern crate rand;
extern crate time;

use std::process;
use std::process::*;
use std::io::*;
use std::thread;
use std::env;
use rand::*;
use std::sync::*;
use std::sync::mpsc::*;
use std::collections::*;
use std::cmp::*;
use time::*;

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() != 5 {
        println!("Usage: ./server <AI1> <AI2> <AI3> <AI4>");
        process::exit(1);
    }
    let mut board: HashMap<String, i64> = HashMap::new();
    for i in 1..5 {
        board.insert(args[i].to_string(), 0);
    }
    println!("$ Game start!");
    let positions = vec![
        [[1, 2, 3, 4], [2, 3, 4, 1], [3, 4, 1, 2], [4, 1, 2, 3]],
        [[1, 3, 2, 4], [3, 2, 4, 1], [2, 4, 1, 3], [4, 1, 3, 2]],
        [[1, 3, 4, 2], [3, 4, 2, 1], [4, 2, 1, 3], [2, 1, 3, 4]],
        [[1, 4, 2, 3], [4, 2, 3, 1], [2, 3, 1, 4], [3, 1, 4, 2]],
        [[1, 4, 3, 2], [4, 3, 2, 1], [3, 2, 1, 4], [2, 1, 4, 3]],
        [[1, 2, 4, 3], [2, 4, 3, 1], [4, 3, 1, 2], [3, 1, 2, 4]],
    ];
    for group in &positions {
        let seed = gen_seed();
        println!("$ Generated new random seeds");
        for position in group {
            for _ in 0..4 {
                println!("$ The positions are {:?}", position);
                let rng = StdRng::from_seed(&seed.clone());
                let paths = [
                    args[position[0]].clone(), args[position[1]].clone(),
                    args[position[2]].clone(), args[position[3]].clone()
                ];
                let mut game = Game::new(paths, rng);
                game.run();
                println!("$ This hand's score: {:?}", game.score);
                let _board = board.clone();
                for i in 0..4 {
                    let score = _board.get(&args[position[i]].to_string()).unwrap();
                    let add = game.score[i];
                    board.insert(args[position[i]].to_string(), score + add);
                    println!("$ AI{} shut", i);
                    let id = game.pids[i];
                    println!("AI{} id={}", i, id);
                    match std::env::consts::OS {
                        "windows" => {
                            Command::new("taskkill")
                                .arg("/PID")
                                .arg(id.to_string())
                                .arg("/F")
                                .arg("/T")
                                .output()
                                .ok();
                        },
                        _ => {
                            Command::new("kill").arg(id.to_string()).output().ok();
                        }
                    }
                }
            }
        }
    }
    println!("$ Final score: {:?}", board);
}

fn gen_seed() -> [usize; 8] {
    let mut seed = [0; 8];
    for i in 0..8 {
        seed[i] = usize::rand(&mut rand::thread_rng());
    }
    seed
}

#[derive(Clone, Debug)]
struct Tiles {
    hands: Vec<String>,
    chows: Vec<String>,
    pungs: Vec<String>,
    kongs: Vec<String>,
    ckongs: Vec<String>,
    cchows: Vec<String>,
    cpungs: Vec<String>
}

struct Game {
    rng: StdRng,
    paths: [String; 4],
    stage: String,
    inputs: Vec<ChildStdin>,
    join_counter: HashSet<usize>,
    action_id: usize,
    tiles: Vec<Tiles>,
    left: Vec<String>,
    last_time: PreciseTime,
    last_tile: String,
    score: [i64; 4],
    messages: HashMap<usize, Message>,
    base: i64,
    pids: [u32; 4]
}

static mut flags: [bool; 4] = [true, true, true, true];

impl Game {
    fn new(paths: [String; 4], rng: StdRng) -> Game {
        let tiles = [
            "1M", "2M", "3M", "4M", "5M", "6M", "7M", "8M", "9M", "1S", "2S", "3S", "4S", "5S",
            "6S", "7S", "8S", "9S", "1T", "2T", "3T", "4T", "5T", "6T", "7T", "8T", "9T", "E", "S",
            "W", "N", "Z", "F", "B", "1M", "2M", "3M", "4M", "5M", "6M", "7M", "8M", "9M", "1S",
            "2S", "3S", "4S", "5S", "6S", "7S", "8S", "9S", "1T", "2T", "3T", "4T", "5T", "6T",
            "7T", "8T", "9T", "E", "S", "W", "N", "Z", "F", "B", "1M", "2M", "3M", "4M", "5M",
            "6M", "7M", "8M", "9M", "1S", "2S", "3S", "4S", "5S", "6S", "7S", "8S", "9S", "1T",
            "2T", "3T", "4T", "5T", "6T", "7T", "8T", "9T", "E", "S", "W", "N", "Z", "F", "B",
            "1M", "2M", "3M", "4M", "5M", "6M", "7M", "8M", "9M", "1S", "2S", "3S", "4S", "5S",
            "6S", "7S", "8S", "9S", "1T", "2T", "3T", "4T", "5T", "6T", "7T", "8T", "9T", "E", "S",
            "W", "N", "Z", "F", "B"
        ];
        let mut left: Vec<_> = tiles.iter().map(|x| x.to_string()).collect();
        rng.clone().shuffle(&mut left);
        unsafe {
            flags = [true, true, true, true];
        }
        Game {
            rng: rng,
            paths: paths,
            stage: "join".to_string(),
            inputs: Vec::new(),
            join_counter: HashSet::new(),
            action_id: 0,
            tiles: Vec::new(),
            left: left,
            last_time: PreciseTime::now(),
            score: [0; 4],
            last_tile: String::new(),
            messages: HashMap::new(),
            base: 4,
            pids: [0, 0, 0, 0]
        }
    }

    fn run(&mut self) {
        let (tx, rx) = mpsc::channel();
        for i in 0..4 {
            let command = Command::new(&self.paths[i])
                              .stdin(Stdio::piped())
                              .stdout(Stdio::piped())
                              .spawn()
                              .unwrap();
            let id = command.id();
            self.pids[i] = id;
            self.inputs.push(command.stdin.unwrap());
            let tx = tx.clone();
            let mut output = BufReader::new(command.stdout.unwrap());
            thread::spawn(move || {
                unsafe {
                    println!("$ AI{} started", i);
                    while flags[i] {
                        let mut result = String::new();
                        output.read_line(&mut result).ok();
                        tx.send(Message { id: i, message: result.trim().to_string() }).ok();
                    }

                }
            });
        }
        loop {
            unsafe {
                if flags == [false, false, false, false] {
                    return;
                }
            }
            if self.stage == "outwait" {
                if self.last_time.to(PreciseTime::now()).num_milliseconds() < 550 {
                    let msg = match rx.try_recv() {
                        Ok(msg) => msg,
                        _ => {
                            thread::sleep_ms(25);
                            continue;
                        }
                    };
                    println!("$ Received message: {:?}", msg);
                    self.messages.insert(msg.id, msg);
                    println!("$ Added to queue!");
                } else {
                    self.outwait();
                }
            } else if self.stage == "qgwait" {
                if self.last_time.to(PreciseTime::now()).num_milliseconds() < 550 {
                    let msg = match rx.try_recv() {
                        Ok(msg) => msg,
                        _ => {
                            thread::sleep_ms(25);
                            continue;
                        }
                    };
                    println!("$ Received message: {:?}", msg);
                    self.messages.insert(msg.id, msg);
                    println!("$ Added to queue!");
                } else {
                    self.qgwait();
                }
            } else {
                let msg = match rx.recv() {
                    Ok(msg) => msg,
                    Err(_) => return
                };
                self.process(msg);
            }
        }
    }

    fn process(&mut self, msg: Message) {
        println!("$ Received message: {:?}", msg);
        let v: Vec<&str> = msg.message.split(' ').collect();
        println!("$ vector = {:?}", v);
        match v[0].trim() {
            "join" => self.join(msg.id),
            "out" => self.out(msg.id, v[1].trim().to_string()),
            "agang" => self.agang(msg.id, v[1].trim().to_string()),
            "jgang" => self.jgang(msg.id, v[1].trim().to_string()),
            "hu" => self.tsumo(msg.id),
            _ => ()
        }
    }

    fn tsumo(&mut self, id: usize) {
        if self.stage != "out" || id != self.action_id {
            println!("$ {} sent invalid hu", id);
            return;
        }
        let time = PreciseTime::now();
        match cal_fan(self.tiles[id].clone(), self.last_tile.clone(), true) {
            Some(x) => {
                let duration = self.last_time.to(time).num_milliseconds();
                if duration >= 1050 {
                    let penalty = (duration - 950) / 100;
                    self.score[id] -= penalty;
                    println!("$ {} was fined {} due to timeout", id, penalty);
                }
                self.score[id] += 3 * (x + self.base);
                for i in 0..4 {
                    if i != id {
                        self.score[i] -= x + self.base;
                    }
                }
                println!("$ {} tsumo!", id);
                unsafe {
                    for i in 0..4 {
                        flags[i] = false;
                    }
                }
            },
            None => {
                //TODO
                println!("$ {} sent invalid hu", id);
                println!("$ {}'s tiles are: {:?}", id, self.tiles[id].hands);
            }
        }
    }

    fn qgwait(&mut self) {
        let mut messages = Vec::new();
        for _msg in self.messages.values() {
            messages.push(_msg.clone());
        }
        messages.sort_by(|a, b| ((a.id + 4 - self.action_id) % 4)
                                    .cmp(&((b.id + 4 - self.action_id) % 4)));
        for msg in messages {
            if msg.id == self.action_id { continue; }
            match msg.message.trim() {
                "qgang" => {
                    if self.hu(msg.id) {
                        println!("$ {} robbed the kong", msg.id);
                        return;
                    }
                },
                _ => ()
            }
        }
        self.pick();
    }

    fn outwait(&mut self) {
        let mut messages = Vec::new();
        for _msg in self.messages.values() {
            messages.push(_msg.clone());
        }
        messages.sort_by(|a, b| {
            let o1 = match a.message.split(' ').next().unwrap() {
                "hu" => (a.id + 4 - self.action_id) % 4,
                "gang" => 16,
                "peng" => 64,
                "chi" => 254,
                _ => 255
            };
            let o2 = match b.message.split(' ').next().unwrap() {
                "hu" => (b.id + 4 - self.action_id) % 4,
                "gang" => 16,
                "peng" => 64,
                "chi" => 254,
                _ => 255
            };
            o1.cmp(&o2)
        });
        let mut sort = "fail";
        let prev_id = self.action_id;
        for msg in messages {
            if msg.id == self.action_id { continue; }
            let v: Vec<&str> = msg.message.split(' ').collect();
            match v[0].trim() {
                "hu" => {
                    if self.hu(msg.id) {
                        println!("{} hu!", msg.id);
                        unsafe {
                            for i in 0..4 {
                                flags[i] = false;
                            }
                        }
                        return;
                    }
                },
                "gang" => {
                    if self.gang(msg.id) {
                        for i in 0..4 {
                            self.inputs[i]
                                .write(format!("mgang {} {}\n", msg.id, self.last_tile.clone())
                                           .to_string()
                                           .as_bytes())
                                .ok();
                            print!("Sent to {}: {}", i,
                                   format!("mgang {} {}\n", msg.id, self.last_tile.clone()));

                            self.inputs[i].flush().ok();
                        }
                        self.action_id = msg.id;
                        sort = "gang";
                        break;
                    }
                },
                "peng" => {
                    if self.peng(msg.id) {
                        for i in 0..4 {
                            self.inputs[i]
                                .write(format!("mpeng {} {}\n", msg.id, self.last_tile.clone())
                                           .to_string()
                                           .as_bytes())
                                .ok();
                            print!("Sent to {}: {}", i,
                                   format!("mpeng {} {}\n", msg.id, self.last_tile.clone()));

                            self.inputs[i].flush().ok();
                        }
                        self.action_id = msg.id;
                        sort = "peng";
                        break;
                    }
                },
                "chi" => {
                    if self.chi(msg.id, v[1].trim()) {
                        for i in 0..4 {
                            self.inputs[i]
                                .write(format!("mchi {} {}\n", msg.id, v[1].trim())
                                           .to_string()
                                           .as_bytes())
                                .ok();
                            print!("Sent to {}: {}", i,
                                   format!("mchi {} {}\n", msg.id, v[1].trim()));

                            self.inputs[i].flush().ok();
                        }
                        self.action_id = msg.id;
                        sort = "chi";
                        break;
                    }
                },
                _ => ()
            }
        }
        if sort != "chi" {
            let post = post_pos(prev_id);
            match self.messages.get(&post) {
                Some(msg) => {
                    if msg.message.split(' ').next().unwrap() == "chi" {
                        println!("$ {} failed to chi", post);
                        self.inputs[post].write("mfail\n".to_string().as_bytes()).ok();
                        print!("Sent to {}: mfail\n", post);

                        self.inputs[post].flush().ok();
                    }
                },
                None => ()
            }
        }
        if sort == "gang" {
            self.pick();
        } else if sort == "fail" {
            self.action_id = post_pos(self.action_id);
            self.pick();
        } else {
            self.stage = "out".to_string();
        }
    }

    fn agang(&mut self, id: usize, tile: String) {
        if self.stage != "out" || id != self.action_id {
            println!("$ {} sent invalid agang", id);
            return;
        }
        if self.tiles[id].hands.iter().filter(|&x| *x == tile).count() == 4 {
            let duration = self.last_time.to(PreciseTime::now()).num_milliseconds();
            if duration >= 1050 {
                let penalty = (duration - 950) / 100;
                self.score[id] -= penalty;
                println!("$ {} was fined {} due to timeout", id, penalty);
            }
            self.tiles[id].hands.retain(|x| x != &tile);
            self.tiles[id].ckongs.push(tile.to_string());
            println!("$ {} gang {} concealedly", id, tile);
            for i in 0..4 {
                self.inputs[i].write(format!("magang {}\n", id).to_string().as_bytes()).ok();
                print!("Sent to {}: {}", i, format!("magang {}\n", id));
                self.inputs[i].flush().ok();
            }
            self.pick();
        } else {
            println!("$ {} sent invalid agang", id);
            println!("$ {}'s tiles are: {:?}", id, self.tiles[id].hands);
        }
    }

    fn jgang(&mut self, id: usize, tile: String) {
        if self.stage != "out" || id != self.action_id {
            return;
        }
        if self.tiles[id].hands.contains(&tile) && self.tiles[id].pungs.contains(&tile) {
            let duration = self.last_time.to(PreciseTime::now()).num_milliseconds();
            if duration >= 1050 {
                let penalty = (duration - 950) / 100;
                self.score[id] -= penalty;
                println!("$ {} was fined {} due to timeout", id, penalty);
            }
            let index = self.tiles[id].hands.iter().position(|x| *x == tile).unwrap();
            self.tiles[id].hands.remove(index);
            let index = self.tiles[id].pungs.iter().position(|x| *x == tile).unwrap();
            self.tiles[id].pungs.remove(index);
            self.tiles[id].kongs.push(tile.clone());
            println!("$ {} gang {} by adding", id, tile);
            for i in 0..4 {
                self
                .inputs[i].write(format!("mjgang {} {}\n", id, tile).to_string().as_bytes()).ok();
                print!("Sent to {}: {}", i, format!("mjgang {} {}\n", id, tile));
                self.inputs[i].flush().ok();
            }
            self.stage = "qgwait".to_string();
            self.last_tile = tile;
            self.messages.clear();
            println!("$ Waiting for action");
            self.last_time = PreciseTime::now();
        }
    }

    fn join(&mut self, id: usize) {
        if self.stage != "join" {
            println!("$ {} sent invalid join message", id);
            return;
        }
        self.join_counter.insert(id);
        println!("$ {} joined", id);
        if self.join_counter.len() == 4 {
            for i in 0..4 {
                self.inputs[i].write(format!("id {}\n", i).as_bytes()).ok();
                print!("Sent to {}: {}", i, format!("id {}\n", i));
                self.inputs[i].flush().ok();
            }
            self.start();
        }
    }

    fn start(&mut self) {
        self.action_id = self.rng.gen_range(0, 4);
        println!("$ {} acts first", self.action_id);
        for i in 0..4 {
            self.inputs[i].write(format!("first {}\n", self.action_id).as_bytes()).ok();
            print!("Sent to {}: {}", i, format!("first {}\n", self.action_id));
            self.inputs[i].flush().ok();
        }
        self.init();
    }

    fn init(&mut self) {
        for i in 0..4 {
            let mut output = "init".to_string();
            self.tiles.push(Tiles {
                hands: Vec::new(),
                chows: Vec::new(),
                pungs: Vec::new(),
                kongs: Vec::new(),
                ckongs: Vec::new(),
                cchows: Vec::new(),
                cpungs: Vec::new()
            });
            print!("$ {}'s first 13 tiles are:", i);
            for _ in 0..13 {
                let tile = self.left.pop().unwrap();
                output.push_str(" ");
                output.push_str(&tile);
                print!("{} ", tile);
                self.tiles[i].hands.push(tile);
            }
            println!("");
            output.push_str("\n");
            self.inputs[i].write(output.as_bytes()).ok();
            print!("Sent to {}: {}", i, output);
            self.inputs[i].flush().ok();
        }
        self.pick();
    }

    fn pick(&mut self) {
        if self.left.len() == 0 {
            self.draw();
            return;
        }
        let tile = self.left.pop().unwrap();
        println!("$ {} picked {}", self.action_id, tile);
        self.tiles[self.action_id].hands.push(tile.clone());
        print!("Sent to {}: {}", self.action_id, format!("pick {}\n", tile));
        self.inputs[self.action_id].write(format!("pick {}\n", tile).to_string().as_bytes()).ok();

        self.inputs[self.action_id].flush().ok();
        for i in 0..4 {
            if i == self.action_id { continue; }
            self
            .inputs[i].write(format!("mpick {}\n", self.action_id).to_string().as_bytes()).ok();
            print!("Sent to {}: {}", i, format!("mpick {}\n", self.action_id));
            self.inputs[i].flush().ok();
        }
        self.stage = "out".to_string();
        self.last_tile = tile;
        println!("$ Waiting for action");
        self.last_time = PreciseTime::now();
    }

    fn out(&mut self, id: usize, tile: String) {
        if self.stage != "out" || id != self.action_id {
            println!("$ {} sent invalid out", id);
            return;
        }
        match self.tiles[id].hands.iter().position(|x| *x == tile) {
            Some(index) => {
                println!("$ {} discarded {}", id, tile);
                let duration = self.last_time.to(PreciseTime::now()).num_milliseconds();
                if duration >= 1050 {
                    let penalty = (duration - 950) / 100;
                    self.score[id] -= penalty;
                    println!("$ {} was fined {} due to timeout", id, penalty);
                }
                self.tiles[id].hands.remove(index);
                for i in 0..4 {
                    if i == self.action_id { continue; }
                    self.inputs[i]
                        .write(format!("mout {} {}\n", self.action_id, tile)
                                   .to_string()
                                   .as_bytes())
                        .ok();
                    print!("Sent to {}: {}", i, format!("mout {} {}\n", self.action_id, tile));
                    self.inputs[i].flush().ok();
                }
                self.last_tile = tile;
                self.stage = "outwait".to_string();
                self.messages.clear();
                self.last_time = PreciseTime::now();
            },
            None => {
                println!("$ {} sent invalid out", id);
                println!("$ {}'s tiles are: {:?}", id, self.tiles[id].hands);
                //TODO
            }
        }
    }

    fn hu(&mut self, id: usize) -> bool {
        match cal_fan(self.tiles[id].clone(), self.last_tile.clone(), false) {
            Some(x) => {
                self.score[id] += 3 * self.base + x;
                for i in 0..4 {
                    if i != id {
                        self.score[i] -= self.base;
                    }
                }
                self.score[self.action_id] -= x;
                return true;
            },
            None => {
                println!("$ {} sent invalid hu", id);
                println!("$ {}'s tiles are: {:?}", id, self.tiles[id].hands);
            }
        }
        return false;
    }

    fn gang(&mut self, id: usize) -> bool {
        let tile = self.last_tile.clone();
        if self.tiles[id].hands.iter().filter(|&x| *x == tile).count() == 3 {
            self.tiles[id].hands.retain(|x| x != &tile);
            println!("$ {} gang {}", id, tile);
            self.tiles[id].kongs.push(tile);
            return true;
        }
        println!("$ {} sent invalid gang", id);
        return false;
    }

    fn peng(&mut self, id: usize) -> bool {
        let tile = self.last_tile.clone();
        if self.tiles[id].hands.iter().filter(|&x| *x == tile).count() >= 2 {
            for _ in 0..2 {
                let index = self.tiles[id].hands.iter().position(|x| *x == tile).unwrap();
                self.tiles[id].hands.remove(index);
            }
            println!("$ {} peng {}", id, tile);
            self.tiles[id].pungs.push(tile);
            return true;
        }
        println!("$ {} sent invalid peng", id);
        return false;
    }

    fn chi(&mut self, id: usize, tile: &str) -> bool {
        if id != post_pos(self.action_id) {
            return false;
        }
        let mut set = HashSet::new();
        set.insert(tile.to_string());
        let second = match post(tile.to_string()) {
            Some(x) => x,
            None => return false
        };
        set.insert(second.clone());
        set.insert(match post(second) {
            Some(x) => x,
            None => return false
        });
        set.remove(&self.last_tile);
        if self.tiles[id].hands.iter().filter(|&x| set.contains(x)).count() >= 2 {
            for i in set {
                let index = self.tiles[id].hands.iter().position(|x| *x == i).unwrap();
                self.tiles[id].hands.remove(index);
            }
            self.tiles[id].chows.push(tile.to_string());
            println!("$ {} chi {}", id, tile);
            return true;
        }
        return false;
    }

    fn draw(&mut self) {
        unsafe {
            println!("$ Draw game!");
            for i in 0..4 {
                flags[i] = false;
            }
        }
    }
}

#[derive(Clone, Debug)]
struct Message {
    id: usize,
    message: String
}

fn post(tile: String) -> Option<String> {
    let chars: Vec<char> = tile.chars().collect();
    if tile.len() == 2 {
        let kind = chars[1];
        if kind == 'M' || kind == 'T' || kind == 'S' {
            let num = match chars[0].to_digit(10) {
                Some(x) => x,
                None => {
                    return None;
                }
            };
            if num < 9 && num > 0 {
                return Some(format!("{}{}", num + 1, kind));
            }
        }
    }
    return None;
}

fn post_pos(pos: usize) -> usize {
    return (pos + 1) % 4;
}

fn combine(_tiles: Tiles) -> Vec<Tiles> {
    let mut v = Vec::new();
    if _tiles.hands.len() == 2 {
        if _tiles.hands[0] == _tiles.hands[1] {
            v.push(_tiles);
            return v;
        }
    }
    let mut tiles = _tiles.clone();
    tiles.hands.sort_by(|a, b| {
        if a.len() == 1 || b.len() == 1 {
            a.cmp(b)
        } else {
            let _a = a.chars().last().unwrap();
            let _b = b.chars().last().unwrap();
            if _a == _b {
                a.cmp(b)
            } else {
                _a.cmp(&_b)
            }
        }
    });
    if _tiles.hands.len() == 14 {
        let mut tile_count = HashMap::new();
        for tile in _tiles.clone().hands {
            if tile_count.contains_key(&tile) {
                let _tile_count = tile_count.clone();
                let count = _tile_count.get(&tile).unwrap();
                tile_count.insert(tile, count + 1);
            } else {
                tile_count.insert(tile, 1);
            }
        }
        //七对
        {
            if tile_count.values().filter(|&x| x % 2 != 0).count() == 0 {
                v.push(_tiles.clone());
            }
        }
        //十三幺
        {
            let mut yao = HashSet::new();
            yao.insert("1M".to_string());
            yao.insert("1S".to_string());
            yao.insert("1T".to_string());
            yao.insert("9M".to_string());
            yao.insert("9S".to_string());
            yao.insert("9T".to_string());
            yao.insert("E".to_string());
            yao.insert("S".to_string());
            yao.insert("W".to_string());
            yao.insert("N".to_string());
            yao.insert("Z".to_string());
            yao.insert("F".to_string());
            yao.insert("B".to_string());
            if _tiles.hands.iter().filter(|&x| !yao.contains(x)).count() == 0 &&
               tile_count.values().filter(|&x| *x > 1).count() == 1 {
                v.push(_tiles.clone());
            }
        }
    }
    let mut last_tile = String::new();
    for t in tiles.hands.clone() {
        if t == last_tile {
            continue;
        }
        let mut _tiles = tiles.clone();
        last_tile = t.clone();
        match post(t.clone()) {
            Some(t2) => {
                match post(t2.clone()) {
                    Some(t3) => {
                        match _tiles.hands.iter().position(|x| *x == t) {
                            Some(index) => {
                                _tiles.hands.remove(index);
                                match _tiles.hands.iter().position(|x| *x == t2) {
                                    Some(index) => {
                                        _tiles.hands.remove(index);
                                        match _tiles.hands.iter().position(|x| *x == t3) {
                                            Some(index) => {
                                                _tiles.hands.remove(index);
                                                _tiles.cchows.push(t.clone());
                                                for x in combine(_tiles) {
                                                    v.push(x);
                                                }
                                            },
                                            _ => ()
                                        }
                                    },
                                    _ => ()
                                }
                            },
                            _ => ()
                        }
                    },
                    _ => ()
                }
            },
            _ => ()
        }
        let mut _tiles = tiles.clone();
        if _tiles.hands.iter().filter(|&x| *x == t.clone()).count() >= 3 {
            for _ in 0..3 {
                let index = tiles.hands.iter().position(|x| *x == t.clone()).unwrap();
                _tiles.hands.remove(index);
            }
            _tiles.cpungs.push(t.clone());
            for x in combine(_tiles) {
                v.push(x);
            }
        }
    }
    return v;
}

fn cal_fan(tiles: Tiles, add: String, tsumo: bool) -> Option<i64> {
    let mut _tiles = tiles.clone();
    if !tsumo {
        _tiles.hands.push(add.clone());
    }
    let combs = combine(_tiles.clone());
    if combs.len() == 0 {
        return None;
    }
    let mut fans = HashSet::new();
    let mut result = -1;
    for comb in combs {
        let (_fans, _result) = _cal_fan(comb, tsumo);
        if _result > result {
            fans = _fans;
            result = _result;
        }
    }
    // 单调将
    {
        if !fans.contains("十三幺") && !fans.contains("七对") {
            let mut tiles = _tiles.clone();
            let _tiles = vec![
                "1M", "2M", "3M", "4M", "5M", "6M", "7M", "8M", "9M", "1S", "2S", "3S", "4S", "5S",
                "6S", "7S", "8S", "9S", "1T", "2T", "3T", "4T", "5T", "6T", "7T", "8T", "9T", "E",
                "S", "W", "N", "Z", "F", "B"
            ];
            let index = tiles.hands.iter().position(|x| *x == add).unwrap();
            tiles.hands.remove(index);
            let mut flag = true;
            for tile in _tiles {
                if tile == add {
                    continue;
                }
                let mut tiles = tiles.clone();
                tiles.hands.push(tile.to_string());
                if combine(tiles).len() > 0 {
                    flag = false;
                    break;
                }
            }
            if flag {
                result += 1;
                fans.insert("单调将".to_string());
            }
        }
    }
    println!("Tiles are: {:?}", tiles);
    print!("Fans are: ");
    for fan in fans {
        print!("{} ", fan);
    }
    println!("");
    println!("Altogether {}!", result);
    return Some(result);
}

fn _cal_fan(tiles: Tiles, tsumo: bool) -> (HashSet<String>, i64) {
    let mut result: i64 = 0;
    let mut fans = HashSet::new();
    let mut all_tiles = tiles.hands.clone();
    for tile in tiles.chows.clone() {
        all_tiles.push(tile.clone());
        let next = post(tile).unwrap();
        all_tiles.push(next.clone());
        all_tiles.push(post(next).unwrap());
    }
    for tile in tiles.pungs.clone() {
        all_tiles.push(tile);
    }
    for tile in tiles.kongs.clone() {
        all_tiles.push(tile);
    }
    for tile in tiles.ckongs.clone() {
        all_tiles.push(tile);
    }
    for tile in tiles.cchows.clone() {
        all_tiles.push(tile.clone());
        let next = post(tile).unwrap();
        all_tiles.push(next.clone());
        all_tiles.push(post(next).unwrap());
    }
    for tile in tiles.cpungs.clone() {
        all_tiles.push(tile);
    }
    let mut all_chows = tiles.chows.clone();
    for chow in tiles.cchows.clone() {
        all_chows.push(chow);
    }
    let mut yao = HashSet::new();
    yao.insert("1M".to_string());
    yao.insert("1S".to_string());
    yao.insert("1T".to_string());
    yao.insert("9M".to_string());
    yao.insert("9S".to_string());
    yao.insert("9T".to_string());
    yao.insert("E".to_string());
    yao.insert("S".to_string());
    yao.insert("W".to_string());
    yao.insert("N".to_string());
    yao.insert("Z".to_string());
    yao.insert("F".to_string());
    yao.insert("B".to_string());
    let mut jian = HashSet::new();
    jian.insert("Z".to_string());
    jian.insert("F".to_string());
    jian.insert("B".to_string());
    let mut all_pungs = tiles.pungs.clone();
    for pung in tiles.kongs.clone() {
        all_pungs.push(pung);
    }
    for pung in tiles.ckongs.clone() {
        all_pungs.push(pung);
    }
    for pung in tiles.cpungs.clone() {
        all_pungs.push(pung);
    }
    let mut feng = HashSet::new();
    feng.insert("E".to_string());
    feng.insert("S".to_string());
    feng.insert("W".to_string());
    feng.insert("N".to_string());
    let mut tile_count = HashMap::new();
    for tile in tiles.clone().hands {
        if tile_count.contains_key(&tile) {
            let _tile_count = tile_count.clone();
            let count = _tile_count.get(&tile).unwrap();
            tile_count.insert(tile, count + 1);
        } else {
            tile_count.insert(tile, 1);
        }
    }
    //十三幺
    {
        if tiles.hands.len() == 14 &&
           all_tiles.iter().filter(|&x| !yao.contains(x)).count() == 0 &&
           tile_count.values().filter(|&x| *x > 1).count() == 1 {
            result += 88;
            fans.insert("十三幺".to_string());
        }
    }
    // 大四喜
    {
        if all_pungs.iter().filter(|&x| feng.contains(x)).count() == 4 {
            result += 88;
            fans.insert("大四喜".to_string());
        }
    }
    // 大三元
    {
        if all_pungs.iter().filter(|&x| jian.contains(x)).count() == 3 {
            result += 88;
            fans.insert("大三元".to_string());
        }
    }
    // 绿一色
    {
        let mut set = HashSet::new();
        set.insert("2S".to_string());
        set.insert("3S".to_string());
        set.insert("4S".to_string());
        set.insert("6S".to_string());
        set.insert("8S".to_string());
        set.insert("F".to_string());
        if all_tiles.iter().filter(|&x| !set.contains(x)).count() == 0 {
            result += 88;
            fans.insert("绿一色".to_string());
        }
    }
    // 四杠
    {
        if tiles.kongs.len() + tiles.ckongs.len() == 4 {
            result += 88;
            fans.insert("四杠".to_string());
        }
    }
    // 小四喜
    {
        let mut set = HashSet::new();
        set.insert("E".to_string());
        set.insert("S".to_string());
        set.insert("W".to_string());
        set.insert("N".to_string());
        if all_pungs.iter().filter(|&x| set.contains(x)).count() == 3 &&
           tiles.hands.iter().filter(|&x| set.contains(x)).count() == 2 {
            result += 64;
            fans.insert("小四喜".to_string());
        }
    }
    // 小三元
    {

        if all_pungs.iter().filter(|&x| jian.contains(x)).count() == 2 &&
           tiles.hands.iter().filter(|&x| jian.contains(x)).count() == 2 {
            result += 64;
            fans.insert("小三元".to_string());
        }
    }
    // 字一色
    {
        if all_tiles.iter().filter(|&x| x.len() == 2).count() == 0 {
            result += 64;
            fans.insert("字一色".to_string());
        }
    }
    // 四暗刻
    {
        if tiles.cpungs.len() + tiles.ckongs.len() == 4 {
            result += 64;
            fans.insert("四暗刻".to_string());
        }
    }
    // 清幺九
    {
        let mut set = HashSet::new();
        set.insert("1M".to_string());
        set.insert("1S".to_string());
        set.insert("1T".to_string());
        set.insert("9M".to_string());
        set.insert("9S".to_string());
        set.insert("9T".to_string());
        if all_tiles.iter().filter(|&x| !set.contains(x)).count() == 0 && tiles.chows.len() == 0 &&
           tiles.cchows.len() == 0 {
            result += 64;
            fans.insert("清幺九".to_string());
        }
    }
    // 一色四同顺
    {
        for chow in all_chows.clone() {
            if all_chows.iter().filter(|&x| *x == chow).count() == 4 {
                result += 48;
                fans.insert("一色四同顺".to_string());
                break;
            }
        }
    }
    // 三杠
    {
        if tiles.kongs.len() + tiles.ckongs.len() == 3 {
            result += 32;
            fans.insert("三杠".to_string());
        }
    }
    // 混幺九
    {
        if !fans.contains("清幺九") && !fans.contains("字一色") {
            if all_tiles.iter().filter(|&x| !yao.contains(x)).count() == 0 &&
               tiles.chows.len() == 0 && tiles.cchows.len() == 0 {
                result += 32;
                fans.insert("混幺九".to_string());
            }
        }
    }
    // 清一色
    {
        let sample: Vec<_> = all_tiles[0].clone().chars().collect();
        if sample.len() == 2 {
            let color = sample[1];
            if all_tiles.iter()
                   .filter(|&x| x.len() != 2 || x.chars().last().unwrap() != color)
                   .count() == 0 {
                result += 24;
                fans.insert("清一色".to_string());
            }
        }
    }
    //七对
    {
        if tiles.hands.len() == 14 && tile_count.values().filter(|&x| x % 2 != 0).count() == 0 {
            result += 24;
            fans.insert("七对".to_string());
        }
    }
    // 一色三同顺
    {
        for chow in all_chows.clone() {
            if all_chows.iter().filter(|&x| *x == chow).count() == 3 {
                result += 24;
                fans.insert("一色三同顺".to_string());
                break;
            }
        }
    }
    // 三同刻
    {
        for pung in all_pungs.clone() {
            if pung.len() == 1 {
                continue;
            }
            let ord = pung.chars().next().unwrap();
            if all_pungs.iter().filter(|&x| x.chars().next().unwrap() == ord).count() == 3 {
                result += 16;
                fans.insert("三同刻".to_string());
                break;
            }
        }
    }
    // 三暗刻
    {
        if tiles.cpungs.len() + tiles.ckongs.len() == 3 {
            result += 16;
            fans.insert("三暗刻".to_string());
        }
    }
    // 三色三同顺
    {
        for chow in all_chows.clone() {
            let ord = chow.chars().next().unwrap();
            if all_chows.contains(&format!("{}M", ord)) &&
               all_chows.contains(&format!("{}S", ord)) &&
               all_chows.contains(&format!("{}T", ord)) {
                result += 8;
                fans.insert("三色三同顺".to_string());
                break;
            }
        }
    }
    // 碰碰和
    {
        if !fans.contains("大四喜") && !fans.contains("四杠") && !fans.contains("字一色") &&
           !fans.contains("四暗刻") && !fans.contains("清幺九") && !fans.contains("混幺九") {
            if tiles.chows.len() + tiles.cchows.len() == 0 && all_pungs.len() > 0 {
                result += 6;
                fans.insert("碰碰和".to_string());
            }
        }
    }
    // 混一色
    {
        if !fans.contains("字一色") && !fans.contains("清一色") {
            let mut all_tiles = all_tiles.clone();
            all_tiles.retain(|x| x.len() == 2);
            let color = all_tiles[0].chars().last().unwrap();
            if all_tiles.iter().filter(|&x| x.chars().last().unwrap() != color).count() == 0 {
                result += 6;
                fans.insert("混一色".to_string());
            }
        }
    }
    // 五门齐
    {
        if !fans.contains("十三幺") {
            let mut set1 = HashSet::new();
            let mut set2 = HashSet::new();
            set1.insert("E".to_string());
            set1.insert("S".to_string());
            set1.insert("W".to_string());
            set1.insert("N".to_string());
            set2.insert("Z".to_string());
            set2.insert("F".to_string());
            set2.insert("B".to_string());
            if all_tiles.iter()
                       .filter(|&x| x.len() == 2 && x.chars().last().unwrap() == 'M')
                       .count() != 0 &&
               all_tiles.iter()
                       .filter(|&x| x.len() == 2 && x.chars().last().unwrap() == 'S')
                       .count() != 0 &&
               all_tiles.iter()
                       .filter(|&x| x.len() == 2 && x.chars().last().unwrap() == 'T')
                       .count() != 0 &&
               all_tiles.iter().filter(|&x| set1.contains(x)).count() != 0 &&
               all_tiles.iter().filter(|&x| set2.contains(x)).count() != 0 {
                result += 6;
                fans.insert("五门齐".to_string());
            }
        }
    }
    // 门前清
    {
        if !fans.contains("十三幺") && !fans.contains("七对") {
            if tiles.chows.len() + tiles.pungs.len() + tiles.kongs.len() == 0 {
                result += 2;
                fans.insert("门前清".to_string());
            }
        }
    }
    // 断幺
    {
        if all_tiles.iter().filter(|&x| yao.contains(x)).count() == 0 {
            result += 2;
            fans.insert("断幺".to_string());
        }
    }
    // 平和
    {
        if tiles.pungs.len() + tiles.kongs.len() + tiles.ckongs.len() + tiles.cpungs.len() == 0 &&
           tiles.chows.len() + tiles.cchows.len() > 0 {
            result += 2;
            fans.insert("平和".to_string());
        }
    }
    // 箭刻
    {
        if !fans.contains("大三元") && !fans.contains("小三元") {
            let count = all_pungs.iter().filter(|&x| jian.contains(x)).count() as i64;
            if count > 0 {
                result += 2 * count;
                if count == 1 {
                    fans.insert("箭刻".to_string());
                } else {
                    fans.insert(format!("箭刻×{}", count).to_string());
                }
            }
        }
    }
    // 暗杠
    {
        let count = tiles.ckongs.len() as i64;
        if count > 0 {
            result += 2 * count;
            if count == 1 {
                fans.insert("暗杠".to_string());
            } else {
                fans.insert(format!("暗杠×{}", count).to_string());
            }
        }
    }
    // 自摸
    {
        if tsumo {
            result += 1;
            fans.insert("自摸".to_string());
        }
    }
    // 一般高
    {
        if !fans.contains("一色四同顺") && !fans.contains("一色三同顺") {
            let mut chow_count = HashMap::new();
            for chow in all_chows.clone() {
                if chow_count.contains_key(&chow) {
                    let _chow_count = chow_count.clone();
                    let count = _chow_count.get(&chow).unwrap();
                    chow_count.insert(chow, count + 1);
                } else {
                    chow_count.insert(chow, 1);
                }
            }
            let count = chow_count.values().filter(|&x| *x == 2).count();
            if count > 0 {
                result += count as i64;
                if count == 1 {
                    fans.insert("一般高".to_string());
                } else {
                    fans.insert(format!("一般高×{}", count).to_string());
                }
            }
        }
    }
    // 喜相逢
    {
        if !fans.contains("三色三同顺") {
            let mut set = HashSet::new();
            for chow in all_chows.clone() {
                set.insert(chow);
            }
            let mut count = 0;
            for chow in set.clone() {
                let ord = chow.chars().next().unwrap();
                let color = chow.chars().last().unwrap();
                if set.iter()
                       .filter(|&x| x.chars().next().unwrap() == ord &&
                                                                  x.chars().last().unwrap() !=
                                                                      color)
                       .count() != 0 {
                    count += 1;
                }
            }
            count /= 2;
            if count > 0 {
                result += count;
                if count == 1 {
                    fans.insert("喜相逢".to_string());
                } else {
                    fans.insert(format!("喜相逢×{}", count).to_string());
                }
            }
        }
    }
    // 明杠
    {
        let count = tiles.kongs.len();
        if count > 0 && count < 3 {
            result += count as i64;
            if count == 1 {
                fans.insert("明杠".to_string());
            } else {
                fans.insert(format!("明杠×{}", count).to_string());
            }
        }
    }
    return (fans, result);
}