use std::{
    collections::VecDeque,
    io,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use terrabox::{Color, Event, FrameClock, Key, Style, Terminal};

const WIDTH: i32 = 40;
const HEIGHT: i32 = 20;
const TICK_MS: u64 = 125;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn moved(self, direction: Direction) -> Self {
        let (dx, dy) = match direction {
            Direction::Up => (0, -1),
            Direction::Down => (0, 1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
        };
        Self { x: self.x + dx, y: self.y + dy }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Direction { Up, Down, Left, Right }

impl Direction {
    fn opposite(self, other: Self) -> bool {
        matches!((self, other),
            (Self::Up, Self::Down) | (Self::Down, Self::Up) |
            (Self::Left, Self::Right) | (Self::Right, Self::Left))
    }
}

struct Snake {
    body: VecDeque<Point>,
    direction: Direction,
    next_direction: Direction,
    food: Option<Point>,
    score: u32,
    paused: bool,
    ended: bool,
    won: bool,
    seed: u64,
}

impl Snake {
    fn new(seed: u64) -> Self {
        let mut body = VecDeque::new();
        body.push_back(Point { x: WIDTH / 2, y: HEIGHT / 2 });
        body.push_back(Point { x: WIDTH / 2 - 1, y: HEIGHT / 2 });
        body.push_back(Point { x: WIDTH / 2 - 2, y: HEIGHT / 2 });
        let mut game = Self {
            body, direction: Direction::Right, next_direction: Direction::Right,
            food: None, score: 0, paused: false, ended: false, won: false,
            seed: seed | 1,
        };
        game.spawn_food();
        game
    }

    fn random(&mut self) -> u64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        self.seed
    }

    fn spawn_food(&mut self) {
        let free = (WIDTH * HEIGHT) as usize - self.body.len();
        if free == 0 {
            self.food = None;
            self.won = true;
            self.ended = true;
            return;
        }
        let mut index = (self.random() % free as u64) as usize;
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let point = Point { x, y };
                if !self.body.contains(&point) {
                    if index == 0 {
                        self.food = Some(point);
                        return;
                    }
                    index -= 1;
                }
            }
        }
    }

    fn turn(&mut self, direction: Direction) {
        // Compare against the direction of the current tick, so rapid
        // keypresses cannot reverse into the snake before it moves.
        if !direction.opposite(self.direction) {
            self.next_direction = direction;
        }
    }

    fn tick(&mut self) {
        if self.paused || self.ended { return; }
        self.direction = self.next_direction;
        let head = self.body[0].moved(self.direction);
        let eating = self.food == Some(head);
        let hits_wall = head.x < 0 || head.y < 0 || head.x >= WIDTH || head.y >= HEIGHT;
        let collides = self.body.iter()
            .take(self.body.len() - usize::from(!eating))
            .any(|&segment| segment == head);
        if hits_wall || collides {
            self.ended = true;
            return;
        }
        self.body.push_front(head);
        if eating {
            self.score += 10;
            self.spawn_food();
        } else {
            self.body.pop_back();
        }
    }
}

fn render(term: &mut Terminal, game: &Snake) -> io::Result<()> {
    let size = term.size();
    term.clear();
    const BOARD_W: u16 = WIDTH as u16 + 2;
    const BOARD_H: u16 = HEIGHT as u16 + 2;
    if size.width < BOARD_W || size.height < BOARD_H + 3 {
        term.put_str(0, 0, "TerraSnake: enlarge the terminal", Style::default());
        term.present()?;
        return Ok(());
    }
    let ox = (size.width - BOARD_W) / 2;
    let oy = (size.height - BOARD_H - 3) / 2 + 2;
    let header = format!("TERRASNAKE  |  SCORE {:04}", game.score);
    term.put_str(ox, oy - 2, &header, Style::new(Color::Indexed(10), Color::Default));
    let border = Style::new(Color::Indexed(6), Color::Default);
    for x in 0..BOARD_W {
        term.put_char(ox + x, oy, '#', border);
        term.put_char(ox + x, oy + BOARD_H - 1, '#', border);
    }
    for y in 1..BOARD_H - 1 {
        term.put_char(ox, oy + y, '#', border);
        term.put_char(ox + BOARD_W - 1, oy + y, '#', border);
    }
    if let Some(food) = game.food {
        term.put_char(ox + food.x as u16 + 1, oy + food.y as u16 + 1,
            '*', Style::new(Color::Indexed(9), Color::Default));
    }
    for (index, segment) in game.body.iter().enumerate() {
        let style = Style::new(Color::Indexed(if index == 0 { 11 } else { 10 }), Color::Default);
        term.put_char(ox + segment.x as u16 + 1, oy + segment.y as u16 + 1,
            if index == 0 { '@' } else { 'o' }, style);
    }
    let message = if game.won { "YOU WIN! R: restart   Q: quit" }
        else if game.ended { "GAME OVER! R: restart   Q: quit" }
        else if game.paused { "PAUSED - Space: resume   Q: quit" }
        else { "Arrows/WASD: move  Space: pause  Q: quit" };
    term.put_str(ox, oy + BOARD_H, message, Style::default());
    term.hide_cursor();
    term.present()
}

fn main() -> io::Result<()> {
    let seed = SystemTime::now().duration_since(UNIX_EPOCH)
        .unwrap_or_default().as_nanos() as u64;
    let mut term = Terminal::new()?;
    let mut game = Snake::new(seed);
    let mut clock = FrameClock::new(Duration::from_millis(TICK_MS), Instant::now())
        .expect("nonzero tick interval");
    let mut redraw = true;
    loop {
        if clock.poll(Instant::now()).is_some() {
            game.tick();
            redraw = true;
        }
        if redraw {
            render(&mut term, &game)?;
            redraw = false;
        }
        let timeout = clock.deadline().map(|d| d.remaining(Instant::now()));
        match term.wait_event(timeout)? {
            Some(Event::Key(k)) => {
                match k.key {
                    Key::Escape | Key::Char('q' | 'Q') => break,
                    Key::Up | Key::Char('w' | 'W') => game.turn(Direction::Up),
                    Key::Down | Key::Char('s' | 'S') => game.turn(Direction::Down),
                    Key::Left | Key::Char('a' | 'A') => game.turn(Direction::Left),
                    Key::Right | Key::Char('d' | 'D') => game.turn(Direction::Right),
                    Key::Char(' ') if !game.ended => {
                        game.paused = !game.paused;
                        if game.paused { clock.pause(); }
                        else { clock.resume(Instant::now()); }
                    }
                    Key::Char('r' | 'R') if game.ended => {
                        game = Snake::new(seed ^ (game.score as u64).wrapping_add(Instant::now().elapsed().as_nanos() as u64).wrapping_add(game.body.len() as u64));
                        clock.resume(Instant::now());
                    }
                    _ => {}
                }
                redraw = true;
            }
            Some(Event::Resize(_)) => redraw = true,
            _ => {}
        }
    }
    Ok(())
}
