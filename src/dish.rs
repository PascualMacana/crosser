//! Plato de Petri: dos orillas y el linaje de decisiones.

use crate::{evolve_on, Champion, Decision, Tick, World};
use std::error::Error;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

const WIDTH: usize = 63;
const HEIGHT: usize = 21;

pub fn run(steps: u32, lambda: u32, seed: u64, delay_ms: u64) -> Result<(), Box<dyn Error>> {
    let _cursor = HideCursor::new();
    let mut last: Option<(u32, Champion, Tick, Vec<Decision>)> = None;

    evolve_on(steps, lambda, seed, |step, champ, tick, lin| {
        if matches!(tick, Tick::Start | Tick::Better | Tick::Solved) {
            for frame in &champ.run.frames {
                let _ = present(step, champ, tick, &frame.world, lin);
                sleep(delay_ms);
            }
            sleep(delay_ms.saturating_mul(2));
            last = Some((step, champ.clone(), tick, lin.to_vec()));
        }
    })?;

    if let Some((step, champ, tick, lin)) = last {
        let _ = present(step, &champ, tick, &champ.run.world, &lin);
        sleep(delay_ms.saturating_mul(4));
    }
    Ok(())
}

fn sleep(ms: u64) {
    if ms > 0 {
        thread::sleep(Duration::from_millis(ms));
    }
}

struct HideCursor;

impl HideCursor {
    fn new() -> Self {
        print!("\x1b[?25l\x1b[2J");
        let _ = io::stdout().flush();
        HideCursor
    }
}

impl Drop for HideCursor {
    fn drop(&mut self) {
        print!("\x1b[?25h\x1b[0m\n");
        let _ = io::stdout().flush();
    }
}

struct Canvas {
    w: usize,
    h: usize,
    ch: Vec<char>,
    fg: Vec<u8>,
}

impl Canvas {
    fn new(w: usize, h: usize) -> Self {
        Self {
            w,
            h,
            ch: vec![' '; w * h],
            fg: vec![0; w * h],
        }
    }

    fn put(&mut self, x: i32, y: i32, ch: char, color: u8) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        if x >= self.w || y >= self.h {
            return;
        }
        let i = y * self.w + x;
        self.ch[i] = ch;
        self.fg[i] = color;
    }

    fn puts(&mut self, x: i32, y: i32, s: &str, color: u8) {
        for (i, ch) in s.chars().enumerate() {
            self.put(x + i as i32, y, ch, color);
        }
    }
}

fn present(
    step: u32,
    champ: &Champion,
    tick: Tick,
    world: &World,
    lineage: &[Decision],
) -> io::Result<()> {
    let mut c = Canvas::new(WIDTH, HEIGHT);
    let color = if champ.run.won {
        36
    } else if matches!(champ.run.end, crate::End::Ate { .. }) {
        31
    } else {
        33
    };
    draw_river(&mut c);
    put_bank(&mut c, 2, 4, &world.left, world.farmer_left, color);
    put_bank(&mut c, 40, 4, &world.right, !world.farmer_left, color);
    let boat_x = if world.farmer_left { 18 } else { 32 };
    c.puts(boat_x, 8, "[@]", color);
    put_lineage(&mut c, 1, 13, lineage);

    let mut out = String::with_capacity(WIDTH * HEIGHT * 8);
    out.push_str("\x1b[H");
    out.push_str("\x1b[0m  cruzante  ·  petri dish\x1b[K\n");
    for y in 0..c.h {
        let mut last = 255_u8;
        for x in 0..c.w {
            let i = y * c.w + x;
            if c.fg[i] != last {
                if c.fg[i] == 0 {
                    out.push_str("\x1b[0m");
                } else {
                    out.push_str(&format!("\x1b[{}m", c.fg[i]));
                }
                last = c.fg[i];
            }
            out.push(c.ch[i]);
        }
        out.push_str("\x1b[0m\x1b[K\n");
    }
    out.push_str("\x1b[0m");
    let st = match tick {
        Tick::Start => "guess",
        Tick::Better if champ.run.won => "crossed",
        Tick::Better => "chasing",
        Tick::Solved => "crossed",
    };
    let plan = crate::emit_plan(&champ.plan);
    let plan = if plan.is_empty() {
        "(vacío)".into()
    } else if plan.len() > 42 {
        format!("{}…", &plan[..41])
    } else {
        plan
    };
    out.push_str(&format!(
        "  {st}  ·  step {step}  ·  legales {}  ·  {}\x1b[K\n",
        champ.run.legal_len(),
        champ.run.summary()
    ));
    out.push_str(&format!("  plan  {plan}\x1b[K\n"));
    out.push_str("  linaje  = kept  x dumped  + wrote\x1b[K\n");
    let mut stdout = io::stdout();
    stdout.write_all(out.as_bytes())?;
    stdout.flush()
}

fn draw_river(c: &mut Canvas) {
    c.puts(2, 2, "izquierda", 90);
    c.puts(48, 2, "derecha", 90);
    for y in 3..12 {
        for x in 24..36 {
            let ch = if (x + y) % 2 == 0 { '~' } else { ' ' };
            c.put(x, y, ch, 36);
        }
    }
}

fn put_bank(c: &mut Canvas, x: i32, y: i32, side: &crate::Side, farmer: bool, color: u8) {
    let mut row = y;
    if farmer {
        c.puts(x, row, "@ barquero", color);
        row += 1;
    }
    for label in side.labels() {
        c.puts(x, row, label, color);
        row += 1;
    }
    if !farmer && side.labels().is_empty() {
        c.puts(x, row, "·", 90);
    }
}

fn put_lineage(c: &mut Canvas, x: i32, y: i32, lineage: &[Decision]) {
    c.puts(x, y, "linaje", 90);
    if lineage.is_empty() {
        c.puts(x + 8, y, "(start)", 90);
        return;
    }
    let shown = lineage.len().min(7);
    let start = lineage.len() - shown;
    for (i, d) in lineage[start..].iter().enumerate() {
        let row = y + 1 + i as i32;
        let line = format!("{:>3} {}", d.step, crate::format_decision(d));
        let mut col = x;
        for (j, ch) in line.chars().enumerate() {
            if col >= WIDTH as i32 - 1 {
                break;
            }
            let color = lineage_color(&line, j);
            c.put(col, row, ch, color);
            col += 1;
        }
    }
}

fn lineage_color(line: &str, idx: usize) -> u8 {
    let bytes: Vec<char> = line.chars().collect();
    let mut mark = 90u8;
    for (i, ch) in bytes.iter().enumerate() {
        if i > idx {
            break;
        }
        match ch {
            '=' => mark = 32,
            'x' => mark = 31,
            '+' => mark = 36,
            ' ' => mark = 90,
            _ => {}
        }
    }
    mark
}

#[cfg(test)]
mod tests {
    use crate::simulate;

    #[test]
    fn start_world_has_everyone_on_the_left() {
        let run = simulate(&[]);
        assert!(run.world.farmer_left);
        assert!(run.world.left.lobo && run.world.left.cabra && run.world.left.col);
        assert!(!run.world.right.lobo);
    }
}
