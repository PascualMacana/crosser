//! Constructor que cruza el río reescribiendo el plan donde fracasa.
//!
//! El evaluador está congelado: lobo, cabra y col. El cerebro es un plan de
//! viajes. Si un viaje es ilegal o alguien se come a alguien, el hijo hereda
//! el prefijo legal y cambia a partir de ahí. `spawn` sin búsqueda copia.

mod dish;

use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

const GENERATION: u32 = 0;
const LINEAGE: &str = "0";
pub(crate) const BRAIN: &str = "";

const GENOME: &[(&str, &str)] = &[
    ("Cargo.toml", include_str!("../Cargo.toml")),
    ("src/main.rs", include_str!("main.rs")),
    ("src/dish.rs", include_str!("dish.rs")),
    ("cell.svg", include_str!("../cell.svg")),
    ("README.md", include_str!("../README.md")),
    (".gitignore", include_str!("../.gitignore")),
    ("LICENSE", include_str!("../LICENSE")),
];

const MAX_PLAN: usize = 12;
const DEFAULT_STEPS: u32 = 80;
const DEFAULT_LAMBDA: u32 = 20;
const DEFAULT_DISH_SEED: u64 = 7;

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        None | Some("help") | Some("-h") | Some("--help") => help(),
        Some("identity") | Some("id") => {
            if let Err(e) = identity() {
                eprintln!("error: {e}");
                process::exit(1);
            }
        }
        Some("eval") => {
            let plan = args.next();
            if let Err(e) = eval_cmd(plan.as_deref()) {
                eprintln!("error: {e}");
                process::exit(1);
            }
        }
        Some("genome") => print_genome(),
        Some("evolve") => {
            if let Err(e) = evolve_cmd(args.collect()) {
                eprintln!("error: {e}");
                process::exit(1);
            }
        }
        Some("spawn") => {
            let dest = args.next().unwrap_or_else(|| {
                eprintln!("uso: cruzante spawn <directorio> [--build] [--force]");
                process::exit(2);
            });
            let mut force = false;
            let mut build = false;
            for flag in args {
                match flag.as_str() {
                    "--force" => force = true,
                    "--build" => build = true,
                    other => {
                        eprintln!("flag desconocida: {other}");
                        process::exit(2);
                    }
                }
            }
            if let Err(e) = spawn_cmd(Path::new(&dest), BRAIN, force, build) {
                eprintln!("error: {e}");
                process::exit(1);
            }
        }
        Some("dish") => {
            let mut steps = DEFAULT_STEPS;
            let mut lambda = DEFAULT_LAMBDA;
            let mut seed = DEFAULT_DISH_SEED;
            let mut delay_ms: u64 = 80;
            let rest: Vec<String> = args.collect();
            let mut it = rest.into_iter();
            while let Some(flag) = it.next() {
                match flag.as_str() {
                    "--steps" => {
                        steps = parse_u32(&next_val(&mut it, "--steps").unwrap_or_else(|e| {
                            eprintln!("error: {e}");
                            process::exit(2);
                        }))
                        .unwrap_or_else(|e| {
                            eprintln!("error: {e}");
                            process::exit(2);
                        });
                    }
                    "--lambda" => {
                        lambda = parse_u32(&next_val(&mut it, "--lambda").unwrap_or_else(|e| {
                            eprintln!("error: {e}");
                            process::exit(2);
                        }))
                        .unwrap_or_else(|e| {
                            eprintln!("error: {e}");
                            process::exit(2);
                        });
                    }
                    "--seed" => {
                        seed = parse_u64(&next_val(&mut it, "--seed").unwrap_or_else(|e| {
                            eprintln!("error: {e}");
                            process::exit(2);
                        }))
                        .unwrap_or_else(|e| {
                            eprintln!("error: {e}");
                            process::exit(2);
                        });
                    }
                    "--delay" => {
                        let v = next_val(&mut it, "--delay").unwrap_or_else(|e| {
                            eprintln!("error: {e}");
                            process::exit(2);
                        });
                        delay_ms = v.parse().unwrap_or_else(|_| {
                            eprintln!("no es un número: {v}");
                            process::exit(2);
                        });
                    }
                    other => {
                        eprintln!("flag desconocida: {other}");
                        process::exit(2);
                    }
                }
            }
            if let Err(e) = dish::run(steps, lambda, seed, delay_ms) {
                eprintln!("error: {e}");
                process::exit(1);
            }
        }
        Some(other) => {
            eprintln!("comando desconocido: {other}\n");
            help();
            process::exit(2);
        }
    }
}

fn help() {
    println!(
        "\
cruzante — el hijo hereda los cruces que no mataron (generación {GENERATION}, linaje {LINEAGE})
plan: {plan}

  cruzante identity              generación, linaje, plan, orillas
  cruzante eval [plan]           simula este plan, o el cerebro actual
  cruzante evolve                busca; el mutante copia el prefijo legal
                                 al final imprime el linaje (= se quedó  x se tiró  + se escribió)
                   --steps N      default {DEFAULT_STEPS}
                   --lambda L     mutantes por paso, default {DEFAULT_LAMBDA}
                   --seed S       rng reproducible
                   --spawn <dir>  hijo con el campeón
                   --build        compila al hijo
                   --force        pisa un hijo anterior
                   --write        pisa src/main.rs de este proyecto
  cruzante dish                  anima las dos orillas y el linaje
                   --steps N      default {DEFAULT_STEPS}
                   --lambda L     default {DEFAULT_LAMBDA}
                   --seed S       default {DEFAULT_DISH_SEED} (demo)
                   --delay MS     ms entre frames (default 80)
  cruzante spawn <dir>           copia el genoma actual (sin buscar)
  cruzante genome                imprime las fuentes embebidas

El río está congelado: lobo, cabra, col, bote para uno.
El linaje cuenta hijas, no generaciones: 0 → 0.1 → 0.1.1.
No se copia por la red. Un solo hijo por corrida.",
        plan = if BRAIN.is_empty() { "(vacío)" } else { BRAIN }
    );
}

fn identity() -> Result<(), Box<dyn Error>> {
    let plan = parse_plan(BRAIN)?;
    let run = simulate(&plan);
    let buds = read_brotes(Path::new("."));
    println!("cruzante");
    println!("generación  {GENERATION}");
    println!("linaje      {LINEAGE}");
    println!("hijas       {buds}");
    println!("próximo     {}", child_lineage(LINEAGE, buds));
    println!(
        "plan        {}",
        if BRAIN.is_empty() {
            "(vacío)".to_string()
        } else {
            emit_plan(&plan)
        }
    );
    println!("legales     {}", run.legal_len());
    println!("estado      {}", run.summary());
    println!("score       {:.4}", score(&run));
    println!("archivos    {}", GENOME.len());
    println!();
    print_banks(&run.world);
    Ok(())
}

fn eval_cmd(plan_arg: Option<&str>) -> Result<(), Box<dyn Error>> {
    let src = plan_arg.unwrap_or(BRAIN);
    let plan = parse_plan(src)?;
    let run = simulate(&plan);
    println!(
        "plan     {}",
        if plan.is_empty() {
            "(vacío)".to_string()
        } else {
            emit_plan(&plan)
        }
    );
    for (i, frame) in run.frames.iter().enumerate() {
        if i == 0 {
            println!("inicio   {}", describe_world(&frame.world));
            continue;
        }
        let cargo = frame.cargo.map(|c| c.name()).unwrap_or("?");
        let arrow = if frame.world.farmer_left { "←" } else { "→" };
        println!(
            "paso {i:>2}  {arrow} {cargo:<5}  {}",
            describe_world(&frame.world)
        );
    }
    println!("estado   {}", run.summary());
    Ok(())
}

fn print_genome() {
    for (i, (name, body)) in GENOME.iter().enumerate() {
        if i > 0 {
            println!();
        }
        println!("===== {name} =====");
        print!("{body}");
        if !body.ends_with('\n') {
            println!();
        }
    }
}

struct EvolveOpts {
    steps: u32,
    lambda: u32,
    seed: u64,
    spawn_dir: Option<PathBuf>,
    build: bool,
    force: bool,
    write: bool,
}

fn parse_evolve_opts(args: Vec<String>) -> Result<EvolveOpts, Box<dyn Error>> {
    let mut opts = EvolveOpts {
        steps: DEFAULT_STEPS,
        lambda: DEFAULT_LAMBDA,
        seed: entropy_seed(),
        spawn_dir: None,
        build: false,
        force: false,
        write: false,
    };
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--steps" => opts.steps = parse_u32(&next_val(&mut it, "--steps")?)?,
            "--lambda" => opts.lambda = parse_u32(&next_val(&mut it, "--lambda")?)?,
            "--seed" => opts.seed = parse_u64(&next_val(&mut it, "--seed")?)?,
            "--spawn" => opts.spawn_dir = Some(PathBuf::from(next_val(&mut it, "--spawn")?)),
            "--build" => opts.build = true,
            "--force" => opts.force = true,
            "--write" => opts.write = true,
            other => return Err(format!("flag desconocida: {other}").into()),
        }
    }
    if opts.build && opts.spawn_dir.is_none() {
        return Err("--build pide --spawn <dir>".into());
    }
    Ok(opts)
}

fn next_val(it: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, Box<dyn Error>> {
    it.next()
        .ok_or_else(|| format!("{flag} pide un valor").into())
}

fn parse_u32(s: &str) -> Result<u32, Box<dyn Error>> {
    s.parse().map_err(|_| format!("no es u32: {s}").into())
}

fn parse_u64(s: &str) -> Result<u64, Box<dyn Error>> {
    s.parse().map_err(|_| format!("no es u64: {s}").into())
}

fn entropy_seed() -> u64 {
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1);
    t ^ ((process::id() as u64) << 32)
}

fn evolve_cmd(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let opts = parse_evolve_opts(args)?;
    let (champ, lineage) = evolve_on(opts.steps, opts.lambda, opts.seed, |step, best, tick, lin| {
        if matches!(tick, Tick::Start | Tick::Better | Tick::Solved) {
            let mark = match tick {
                Tick::Start => "",
                Tick::Better if best.run.won => "  llegó",
                Tick::Better => "  *",
                Tick::Solved => "  llegó",
            };
            println!(
                "paso {step:>4}  legales {:>2}  {}  {}{mark}",
                best.run.legal_len(),
                best.run.summary(),
                emit_plan(&best.plan)
            );
            if let Some(d) = lin.last() {
                if !matches!(tick, Tick::Start) {
                    println!("           {}", format_decision(d));
                }
            }
        }
    })?;
    print_lineage(&lineage);

    println!();
    println!("seed       {}", opts.seed);
    println!("campeón    {}", emit_plan(&champ.plan));
    println!("legales    {}", champ.run.legal_len());
    println!("estado     {}", champ.run.summary());
    println!(
        "resultado   {}",
        if champ.run.won {
            "cruzaron"
        } else {
            "no cruzaron"
        }
    );

    let brain = emit_plan(&champ.plan);
    if opts.write {
        write_local_brain(&brain)?;
    }
    if let Some(dir) = opts.spawn_dir {
        spawn_cmd(&dir, &brain, opts.force, opts.build)?;
    } else if !opts.write {
        println!("(nada escrito: usá --spawn <dir> o --write)");
    }
    Ok(())
}

#[cfg(test)]
fn evolve(steps: u32, lambda: u32, seed: u64) -> Result<Champion, Box<dyn Error>> {
    Ok(evolve_with_lineage(steps, lambda, seed)?.0)
}

#[cfg(test)]
fn evolve_with_lineage(
    steps: u32,
    lambda: u32,
    seed: u64,
) -> Result<(Champion, Vec<Decision>), Box<dyn Error>> {
    evolve_on(steps, lambda, seed, |step, best, tick, lin| {
        if matches!(tick, Tick::Start | Tick::Better | Tick::Solved) {
            let mark = match tick {
                Tick::Start => "",
                Tick::Better if best.run.won => "  llegó",
                Tick::Better => "  *",
                Tick::Solved => "  llegó",
            };
            println!(
                "paso {step:>4}  legales {:>2}  {}  {}{mark}",
                best.run.legal_len(),
                best.run.summary(),
                emit_plan(&best.plan)
            );
            if let Some(d) = lin.last() {
                if !matches!(tick, Tick::Start) {
                    println!("           {}", format_decision(d));
                }
            }
        }
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tick {
    Start,
    Better,
    Solved,
}

pub(crate) fn evolve_on<F>(
    steps: u32,
    lambda: u32,
    seed: u64,
    mut hook: F,
) -> Result<(Champion, Vec<Decision>), Box<dyn Error>>
where
    F: FnMut(u32, &Champion, Tick, &[Decision]),
{
    let mut rng = Rng::new(seed);
    let plan = parse_plan(BRAIN)?;
    let mut best = Champion::from_plan(plan);
    let mut lineage: Vec<Decision> = Vec::new();
    hook(0, &best, Tick::Start, &lineage);
    for step in 1..=steps {
        let mut winner = best.clone();
        for _ in 0..lambda {
            let m = mutate(&best.plan, &best.run, &mut rng);
            if m.len() > MAX_PLAN {
                continue;
            }
            let cand = Champion::from_plan(m);
            if cand.score < winner.score {
                winner = cand;
            }
        }
        if winner.score < best.score {
            let solved = winner.run.won && !best.run.won;
            lineage.push(decide(step, &best.plan, &winner.plan));
            best = winner;
            hook(
                step,
                &best,
                if solved { Tick::Solved } else { Tick::Better },
                &lineage,
            );
        }
    }
    Ok((best, lineage))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Decision {
    pub step: u32,
    pub kept: Vec<Cargo>,
    pub discarded: Option<Cargo>,
    pub added: Vec<Cargo>,
}

pub(crate) fn common_len(a: &[Cargo], b: &[Cargo]) -> usize {
    a.iter().zip(b).take_while(|(x, y)| x == y).count()
}

pub(crate) fn decide(step: u32, old: &[Cargo], new: &[Cargo]) -> Decision {
    let k = common_len(old, new);
    Decision {
        step,
        kept: new[..k].to_vec(),
        discarded: old.get(k).copied(),
        added: new[k..].to_vec(),
    }
}

pub(crate) fn format_decision(d: &Decision) -> String {
    let mut parts = Vec::new();
    for c in &d.kept {
        parts.push(format!("={}", c.name()));
    }
    if let Some(c) = d.discarded {
        parts.push(format!("x{}", c.name()));
    }
    for c in &d.added {
        parts.push(format!("+{}", c.name()));
    }
    if parts.is_empty() {
        "(vacío)".into()
    } else {
        parts.join(" ")
    }
}

pub(crate) fn print_lineage(lineage: &[Decision]) {
    println!();
    println!("linaje     ·  = se quedó  x se tiró  + se escribió");
    if lineage.is_empty() {
        println!("           (sin decisiones)");
        return;
    }
    for d in lineage {
        println!("  {:>4}  {}", d.step, format_decision(d));
    }
}

fn write_local_brain(brain: &str) -> Result<(), Box<dyn Error>> {
    let main_path = Path::new("src/main.rs");
    if !looks_like_cruzante(Path::new(".")) {
        return Err("este directorio no parece un cruzante (corrés desde el crate)".into());
    }
    let src = fs::read_to_string(main_path)?;
    let next = patch_const_str(&src, "BRAIN", brain)?;
    fs::write(main_path, next)?;
    println!("escribió plan en {}", main_path.display());
    println!("compilá de nuevo para que el binario nazca con él");
    Ok(())
}

const BROTES: &str = ".brotes";

pub(crate) fn child_lineage(parent: &str, buds: u32) -> String {
    format!("{parent}.{}", buds + 1)
}

fn spawn_cmd(dest: &Path, brain: &str, force: bool, build: bool) -> Result<(), Box<dyn Error>> {
    let dest = normalize_dest(dest)?;
    let (lineage, record) = plan_birth(Path::new("."), LINEAGE, &dest, force)?;
    spawn_lineage(&dest, brain, force, build, &lineage)?;
    if record {
        record_birth(Path::new("."))?;
    }
    Ok(())
}

#[cfg(test)]
fn spawn(dest: &Path, brain: &str, force: bool, build: bool) -> Result<(), Box<dyn Error>> {
    spawn_lineage(dest, brain, force, build, &child_lineage(LINEAGE, 0))
}

fn spawn_lineage(
    dest: &Path,
    brain: &str,
    force: bool,
    build: bool,
    next_lineage: &str,
) -> Result<(), Box<dyn Error>> {
    let dest = normalize_dest(dest)?;
    assert_safe_dest(&dest)?;
    prepare_dest(&dest, force)?;

    let next_gen = GENERATION + 1;
    let child_main = rewrite_main(include_str!("main.rs"), next_gen, next_lineage, brain)?;

    for (rel, contents) in GENOME {
        let body = if *rel == "src/main.rs" {
            child_main.clone()
        } else {
            (*contents).to_string()
        };
        let path = dest.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, body)?;
        println!("escribió {}", path.display());
    }

    println!(
        "hijo generación {next_gen} linaje {next_lineage} plan {brain} → {}",
        dest.display()
    );

    if build {
        let status = Command::new("cargo")
            .arg("build")
            .current_dir(&dest)
            .status()?;
        if !status.success() {
            return Err("cargo build del hijo falló".into());
        }
        println!("hijo compilado: {}/target/debug/cruzante", dest.display());
    }
    Ok(())
}

fn rewrite_main(
    src: &str,
    generation: u32,
    lineage: &str,
    brain: &str,
) -> Result<String, Box<dyn Error>> {
    let src = patch_const_u32(src, "GENERATION", generation)?;
    let src = patch_const_str(&src, "LINEAGE", lineage)?;
    patch_const_str(&src, "BRAIN", brain)
}

fn patch_const_u32(src: &str, name: &str, new: u32) -> Result<String, Box<dyn Error>> {
    let start_pat = format!("const {name}: u32 = ");
    let start = src
        .find(&start_pat)
        .ok_or_else(|| format!("no encuentro {name} en el genoma"))?;
    let value_start = start + start_pat.len();
    let rel_end = src[value_start..]
        .find(';')
        .ok_or_else(|| format!("const {name} sin cierre"))?;
    let mut out = String::with_capacity(src.len() + 8);
    out.push_str(&src[..value_start]);
    out.push_str(&new.to_string());
    out.push_str(&src[value_start + rel_end..]);
    Ok(out)
}

fn patch_const_str(src: &str, name: &str, new_val: &str) -> Result<String, Box<dyn Error>> {
    if new_val.contains('"') || new_val.contains('\\') {
        return Err("el valor no puede tener comillas ni backslash".into());
    }
    let start_pat = format!("const {name}: &str = \"");
    let start = src
        .find(&start_pat)
        .ok_or_else(|| format!("no encuentro {name} en el genoma"))?;
    let value_start = start + start_pat.len();
    let rel_end = src[value_start..]
        .find('"')
        .ok_or_else(|| format!("const {name} sin cierre"))?;
    let value_end = value_start + rel_end;
    let mut out = String::with_capacity(src.len() + new_val.len());
    out.push_str(&src[..value_start]);
    out.push_str(new_val);
    out.push_str(&src[value_end..]);
    Ok(out)
}

fn read_const_str(dir: &Path, name: &str) -> Option<String> {
    let src = fs::read_to_string(dir.join("src/main.rs")).ok()?;
    let start_pat = format!("const {name}: &str = \"");
    let start = src.find(&start_pat)?;
    let value_start = start + start_pat.len();
    let rel_end = src[value_start..].find('"')?;
    Some(src[value_start..value_start + rel_end].to_string())
}

fn read_brotes(dir: &Path) -> u32 {
    fs::read_to_string(dir.join(BROTES))
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

fn record_birth(parent: &Path) -> Result<(), Box<dyn Error>> {
    if !looks_like_cruzante(parent) {
        return Ok(());
    }
    let n = read_brotes(parent) + 1;
    fs::write(parent.join(BROTES), format!("{n}\n"))?;
    println!(
        "padre  hijas {n}  → próximo linaje {}",
        child_lineage(LINEAGE, n)
    );
    Ok(())
}

fn plan_birth(
    parent: &Path,
    parent_lin: &str,
    dest: &Path,
    force: bool,
) -> Result<(String, bool), Box<dyn Error>> {
    if force && dest.exists() && looks_like_cruzante(dest) {
        if let Some(lin) = read_const_str(dest, "LINEAGE") {
            return Ok((lin, false));
        }
    }
    Ok((child_lineage(parent_lin, read_brotes(parent)), true))
}

fn normalize_dest(dest: &Path) -> Result<PathBuf, Box<dyn Error>> {
    if dest.as_os_str().is_empty() {
        return Err("directorio vacío".into());
    }
    if dest.is_absolute() {
        Ok(dest.to_path_buf())
    } else {
        Ok(env::current_dir()?.join(dest))
    }
}

fn assert_safe_dest(dest: &Path) -> Result<(), Box<dyn Error>> {
    let cwd = env::current_dir()?;
    if dest == cwd {
        return Err("no voy a sobreescribir el directorio actual".into());
    }
    let home = env::var_os("HOME").map(PathBuf::from);
    let forbidden = [
        PathBuf::from("/"),
        PathBuf::from("/usr"),
        PathBuf::from("/bin"),
        PathBuf::from("/sbin"),
        PathBuf::from("/etc"),
        PathBuf::from("/System"),
        PathBuf::from("/Library"),
        PathBuf::from("/Applications"),
        PathBuf::from("/private"),
    ];
    for p in &forbidden {
        if dest == p {
            return Err(format!("destino prohibido: {}", dest.display()).into());
        }
    }
    if let Some(home) = &home {
        if dest == home {
            return Err("no voy a escribir en $HOME".into());
        }
    }
    Ok(())
}

fn prepare_dest(dest: &Path, force: bool) -> Result<(), Box<dyn Error>> {
    if !dest.exists() {
        fs::create_dir_all(dest)?;
        return Ok(());
    }
    if dest.is_file() {
        return Err(format!("{} es un archivo", dest.display()).into());
    }
    let empty = dest.read_dir()?.next().is_none();
    if empty {
        return Ok(());
    }
    if !force {
        return Err(format!(
            "{} ya existe y no está vacío (usa --force si es un cruzante anterior)",
            dest.display()
        )
        .into());
    }
    if !looks_like_cruzante(dest) {
        return Err(format!("{} no parece un cruzante; no lo borro", dest.display()).into());
    }
    fs::remove_dir_all(dest)?;
    fs::create_dir_all(dest)?;
    Ok(())
}

fn looks_like_cruzante(dir: &Path) -> bool {
    let cargo = fs::read_to_string(dir.join("Cargo.toml")).unwrap_or_default();
    let main = fs::read_to_string(dir.join("src/main.rs")).unwrap_or_default();
    cargo.contains("name = \"cruzante\"")
        && main.contains("const GENERATION:")
        && main.contains("const BRAIN:")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Cargo {
    Nada,
    Lobo,
    Cabra,
    Col,
}

impl Cargo {
    fn name(self) -> &'static str {
        match self {
            Cargo::Nada => "nada",
            Cargo::Lobo => "lobo",
            Cargo::Cabra => "cabra",
            Cargo::Col => "col",
        }
    }

    fn parse(s: &str) -> Result<Self, Box<dyn Error>> {
        match s {
            "nada" => Ok(Cargo::Nada),
            "lobo" => Ok(Cargo::Lobo),
            "cabra" => Ok(Cargo::Cabra),
            "col" => Ok(Cargo::Col),
            other => Err(format!("carga desconocida: {other}").into()),
        }
    }
}

const ALL_CARGO: [Cargo; 4] = [Cargo::Nada, Cargo::Lobo, Cargo::Cabra, Cargo::Col];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Side {
    pub lobo: bool,
    pub cabra: bool,
    pub col: bool,
}

impl Side {
    fn empty() -> Self {
        Self {
            lobo: false,
            cabra: false,
            col: false,
        }
    }

    fn full() -> Self {
        Self {
            lobo: true,
            cabra: true,
            col: true,
        }
    }

    fn has(&self, cargo: Cargo) -> bool {
        match cargo {
            Cargo::Nada => true,
            Cargo::Lobo => self.lobo,
            Cargo::Cabra => self.cabra,
            Cargo::Col => self.col,
        }
    }

    fn set(&mut self, cargo: Cargo, on: bool) {
        match cargo {
            Cargo::Nada => {}
            Cargo::Lobo => self.lobo = on,
            Cargo::Cabra => self.cabra = on,
            Cargo::Col => self.col = on,
        }
    }

    fn labels(&self) -> Vec<&'static str> {
        let mut v = Vec::new();
        if self.lobo {
            v.push("lobo");
        }
        if self.cabra {
            v.push("cabra");
        }
        if self.col {
            v.push("col");
        }
        v
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct World {
    pub left: Side,
    pub right: Side,
    pub farmer_left: bool,
}

impl World {
    fn start() -> Self {
        Self {
            left: Side::full(),
            right: Side::empty(),
            farmer_left: true,
        }
    }

    fn won(&self) -> bool {
        !self.farmer_left && self.right == Side::full() && self.left == Side::empty()
    }

    fn here(&self) -> &Side {
        if self.farmer_left {
            &self.left
        } else {
            &self.right
        }
    }

    fn here_mut(&mut self) -> &mut Side {
        if self.farmer_left {
            &mut self.left
        } else {
            &mut self.right
        }
    }

    fn key(&self) -> u8 {
        let mut k = 0u8;
        if self.farmer_left {
            k |= 1;
        }
        if self.left.lobo {
            k |= 2;
        }
        if self.left.cabra {
            k |= 4;
        }
        if self.left.col {
            k |= 8;
        }
        k
    }
}

fn eat_reason(side: &Side, farmer_here: bool) -> Option<&'static str> {
    if farmer_here {
        return None;
    }
    if side.lobo && side.cabra {
        return Some("el lobo se come la cabra");
    }
    if side.cabra && side.col {
        return Some("la cabra se come el col");
    }
    None
}

#[derive(Clone, Debug)]
pub(crate) struct Frame {
    pub world: World,
    pub cargo: Option<Cargo>,
}

#[derive(Clone, Debug)]
pub(crate) enum End {
    Won,
    Ate { at: usize, reason: &'static str },
    Invalid { at: usize, reason: &'static str },
    Unfinished,
}

#[derive(Clone, Debug)]
pub(crate) struct Run {
    pub plan: Vec<Cargo>,
    pub frames: Vec<Frame>,
    pub world: World,
    pub end: End,
    pub won: bool,
}

impl Run {
    pub(crate) fn legal_len(&self) -> usize {
        match self.end {
            End::Won => self.plan.len(),
            End::Ate { at, .. } | End::Invalid { at, .. } => at,
            End::Unfinished => self.plan.len(),
        }
    }

    pub(crate) fn cut(&self) -> usize {
        self.legal_len()
    }

    pub(crate) fn summary(&self) -> String {
        match self.end {
            End::Won => "cruzaron".into(),
            End::Ate { at, reason } => format!("paso {}: {reason}", at + 1),
            End::Invalid { at, reason } => format!("paso {}: {reason}", at + 1),
            End::Unfinished => "no terminaron de cruzar".into(),
        }
    }
}

pub(crate) fn simulate(plan: &[Cargo]) -> Run {
    let mut world = World::start();
    let mut frames = vec![Frame {
        world: world.clone(),
        cargo: None,
    }];
    for (i, cargo) in plan.iter().copied().enumerate() {
        if cargo != Cargo::Nada && !world.here().has(cargo) {
            return Run {
                plan: plan.to_vec(),
                frames,
                world,
                end: End::Invalid {
                    at: i,
                    reason: match cargo {
                        Cargo::Lobo => "el lobo no está en esta orilla",
                        Cargo::Cabra => "la cabra no está en esta orilla",
                        Cargo::Col => "el col no está en esta orilla",
                        Cargo::Nada => unreachable!(),
                    },
                },
                won: false,
            };
        }
        world.here_mut().set(cargo, false);
        world.farmer_left = !world.farmer_left;
        world.here_mut().set(cargo, true);
        frames.push(Frame {
            world: world.clone(),
            cargo: Some(cargo),
        });
        if let Some(reason) = eat_reason(&world.left, world.farmer_left) {
            return Run {
                plan: plan.to_vec(),
                frames,
                world,
                end: End::Ate { at: i, reason },
                won: false,
            };
        }
        if let Some(reason) = eat_reason(&world.right, !world.farmer_left) {
            return Run {
                plan: plan.to_vec(),
                frames,
                world,
                end: End::Ate { at: i, reason },
                won: false,
            };
        }
        if world.won() {
            return Run {
                plan: plan.to_vec(),
                frames,
                world,
                end: End::Won,
                won: true,
            };
        }
    }
    Run {
        plan: plan.to_vec(),
        frames,
        world,
        end: End::Unfinished,
        won: false,
    }
}

fn unique_states(run: &Run) -> usize {
    let mut seen = [false; 16];
    let mut n = 0;
    for frame in &run.frames {
        let k = frame.world.key() as usize;
        if !seen[k] {
            seen[k] = true;
            n += 1;
        }
    }
    n
}

fn score(run: &Run) -> f64 {
    if run.won {
        run.plan.len() as f64
    } else {
        200.0 - 10.0 * unique_states(run) as f64 + 0.5 * run.plan.len() as f64
    }
}

#[derive(Clone)]
pub(crate) struct Champion {
    pub(crate) plan: Vec<Cargo>,
    pub(crate) run: Run,
    score: f64,
}

impl Champion {
    fn from_plan(plan: Vec<Cargo>) -> Self {
        let run = simulate(&plan);
        let score = score(&run);
        Self { plan, run, score }
    }
}

pub(crate) fn parse_plan(src: &str) -> Result<Vec<Cargo>, Box<dyn Error>> {
    let src = src.trim();
    if src.is_empty() {
        return Ok(Vec::new());
    }
    src.split_whitespace().map(Cargo::parse).collect()
}

pub(crate) fn emit_plan(plan: &[Cargo]) -> String {
    if plan.is_empty() {
        return String::new();
    }
    plan.iter()
        .map(|c| c.name())
        .collect::<Vec<_>>()
        .join(" ")
}

fn describe_world(world: &World) -> String {
    let mut left = world.left.labels();
    let mut right = world.right.labels();
    if world.farmer_left {
        left.insert(0, "@");
    } else {
        right.insert(0, "@");
    }
    let left = if left.is_empty() {
        "·".into()
    } else {
        left.join(" ")
    };
    let right = if right.is_empty() {
        "·".into()
    } else {
        right.join(" ")
    };
    format!("{left:<22} | {right}")
}

fn print_banks(world: &World) {
    println!("orillas    izquierda                 | derecha");
    println!("           {}", describe_world(world));
}

fn mutate(plan: &[Cargo], run: &Run, rng: &mut Rng) -> Vec<Cargo> {
    if run.won && !plan.is_empty() && rng.frac() < 0.35 {
        let mut p = plan.to_vec();
        p.remove(rng.usize(p.len()));
        return p;
    }
    let mut cut = run.cut().min(plan.len());
    if cut >= MAX_PLAN && cut > 0 {
        cut = rng.usize(cut);
    }
    let mut p = plan[..cut].to_vec();
    let mut cargo = ALL_CARGO[rng.usize(ALL_CARGO.len())];
    if cut < plan.len() && cargo == plan[cut] {
        cargo = ALL_CARGO[rng.usize(ALL_CARGO.len())];
    }
    if p.len() < MAX_PLAN {
        p.push(cargo);
    }
    if rng.frac() < 0.3 && p.len() < MAX_PLAN {
        p.push(ALL_CARGO[rng.usize(ALL_CARGO.len())]);
    }
    p
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    fn frac(&mut self) -> f64 {
        (self.next() >> 11) as f64 / ((1u64 << 53) as f64)
    }

    fn usize(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() as usize) % n
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLASSIC: &str = "cabra nada lobo cabra col nada cabra";

    #[test]
    fn genome_lists_the_project() {
        let names: Vec<_> = GENOME.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"src/main.rs"));
        assert!(names.contains(&"src/dish.rs"));
        assert!(names.contains(&"cell.svg"));
        assert!(names.contains(&"Cargo.toml"));
        assert!(names.contains(&"README.md"));
    }

    #[test]
    fn parse_roundtrip() {
        let p = parse_plan(CLASSIC).unwrap();
        assert_eq!(emit_plan(&p), CLASSIC);
    }

    #[test]
    fn empty_plan_is_unfinished() {
        let run = simulate(&[]);
        assert!(!run.won);
        matches_end_unfinished(&run);
        assert_eq!(run.legal_len(), 0);
    }

    fn matches_end_unfinished(run: &Run) {
        assert!(matches!(run.end, End::Unfinished));
    }

    #[test]
    fn taking_the_wolf_first_leaves_the_goat_to_eat() {
        let run = simulate(&parse_plan("lobo").unwrap());
        assert!(!run.won);
        assert_eq!(run.legal_len(), 0);
        match run.end {
            End::Ate { at, reason } => {
                assert_eq!(at, 0);
                assert!(reason.contains("cabra") && reason.contains("col"));
            }
            other => panic!("expected eat, got {other:?}"),
        }
    }

    #[test]
    fn classic_plan_wins() {
        let run = simulate(&parse_plan(CLASSIC).unwrap());
        assert!(run.won);
        assert_eq!(run.plan.len(), 7);
        assert!(run.world.won());
    }

    #[test]
    fn other_classic_also_wins() {
        let run = simulate(&parse_plan("cabra nada col cabra lobo nada cabra").unwrap());
        assert!(run.won);
    }

    #[test]
    fn missing_cargo_is_invalid() {
        let run = simulate(&parse_plan("cabra lobo").unwrap());
        assert!(!run.won);
        assert_eq!(run.legal_len(), 1);
        match run.end {
            End::Invalid { at, reason } => {
                assert_eq!(at, 1);
                assert!(reason.contains("lobo"));
            }
            other => panic!("expected invalid, got {other:?}"),
        }
    }

    #[test]
    fn mutate_keeps_the_legal_prefix() {
        let plan = parse_plan("cabra lobo").unwrap();
        let run = simulate(&plan);
        assert_eq!(run.cut(), 1);
        let mut rng = Rng::new(3);
        for _ in 0..20 {
            let m = mutate(&plan, &run, &mut rng);
            assert_eq!(m.first().copied(), Some(Cargo::Cabra));
        }
    }

    #[test]
    fn decide_keeps_prefix_and_marks_the_killed_trip() {
        let old = parse_plan("cabra lobo").unwrap();
        let new = parse_plan("cabra nada").unwrap();
        let d = decide(4, &old, &new);
        assert_eq!(d.step, 4);
        assert_eq!(d.kept, vec![Cargo::Cabra]);
        assert_eq!(d.discarded, Some(Cargo::Lobo));
        assert_eq!(d.added, vec![Cargo::Nada]);
        assert_eq!(format_decision(&d), "=cabra xlobo +nada");
    }

    #[test]
    fn decide_append_has_no_discard() {
        let old = parse_plan("cabra").unwrap();
        let new = parse_plan("cabra nada").unwrap();
        let d = decide(2, &old, &new);
        assert!(d.discarded.is_none());
        assert_eq!(d.kept, vec![Cargo::Cabra]);
        assert_eq!(d.added, vec![Cargo::Nada]);
    }

    #[test]
    fn lineage_counts_daughters_not_generations() {
        assert_eq!(child_lineage("0", 0), "0.1");
        assert_eq!(child_lineage("0", 1), "0.2");
        assert_eq!(child_lineage("0.1", 0), "0.1.1");
        assert_ne!(child_lineage("0.1", 0), "0.1.2");
    }

    #[test]
    fn rewrite_patches_plan_and_generation() {
        let next = rewrite_main(include_str!("main.rs"), 1, "0.1", "cabra").unwrap();
        assert!(next.contains("const GENERATION: u32 = 1;"));
        assert!(next.contains("const LINEAGE: &str = \"0.1\";"));
        assert!(next.contains("const BRAIN: &str = \"cabra\";"));
    }

    #[test]
    fn grandchild_lineage_is_not_generation() {
        let child = rewrite_main(
            include_str!("main.rs"),
            1,
            &child_lineage(LINEAGE, 0),
            BRAIN,
        )
        .unwrap();
        let grand = rewrite_main(&child, 2, &child_lineage("0.1", 0), BRAIN).unwrap();
        assert!(grand.contains("const LINEAGE: &str = \"0.1.1\";"));
        assert!(!grand.contains("const LINEAGE: &str = \"0.1.2\";"));
    }

    #[test]
    fn spawn_writes_child_with_plan() {
        let dir = env::temp_dir().join(format!("cruzante-test-{}", process::id()));
        let _ = fs::remove_dir_all(&dir);
        spawn(&dir, "cabra nada", false, false).unwrap();
        let child = fs::read_to_string(dir.join("src/main.rs")).unwrap();
        assert!(child.contains("const BRAIN: &str = \"cabra nada\";"));
        assert!(child.contains("const GENERATION: u32 = 1;"));
        assert!(child.contains("const LINEAGE: &str = \"0.1\";"));
        assert!(dir.join("src/dish.rs").exists());
        assert!(dir.join("cell.svg").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn refuses_to_spawn_over_cwd() {
        let cwd = env::current_dir().unwrap();
        let err = spawn(&cwd, BRAIN, true, false).unwrap_err();
        assert!(err.to_string().contains("directorio actual"));
    }

    #[test]
    fn goat_shuttle_is_not_progress() {
        let short = simulate(&parse_plan("cabra").unwrap());
        let shuttle = simulate(&parse_plan("cabra cabra cabra").unwrap());
        assert!(score(&short) < score(&shuttle));
        assert!(unique_states(&short) <= unique_states(&shuttle));
    }

    #[test]
    fn seed_7_crosses() {
        let champ = evolve(80, 20, DEFAULT_DISH_SEED).unwrap();
        assert!(champ.run.won, "plan {}", emit_plan(&champ.plan));
        assert!(champ.plan.len() <= 11);
    }

    #[test]
    fn seed_7_lineage_records_keep_and_discard() {
        let (champ, lin) = evolve_with_lineage(80, 20, DEFAULT_DISH_SEED).unwrap();
        assert!(champ.run.won);
        assert!(!lin.is_empty());
        assert!(lin.iter().any(|d| d.discarded.is_some()));
        assert!(lin.iter().any(|d| !d.kept.is_empty() || !d.added.is_empty()));
    }
}
