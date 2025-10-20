use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

#[derive(Default)]
struct Dsu {
    parent: Vec<usize>,
    rank: Vec<u8>,
    size: Vec<usize>,
}
impl Dsu {
    fn new() -> Self { Self { parent: Vec::new(), rank: Vec::new(), size: Vec::new() } }
    fn add_node(&mut self) -> usize {
        let id = self.parent.len();
        self.parent.push(id);
        self.rank.push(0);
        self.size.push(1);
        id
    }
    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            let root = self.find(self.parent[x]);
            self.parent[x] = root;
        }
        self.parent[x]
    }
    fn union(&mut self, a: usize, b: usize) {
        let mut ra = self.find(a);
        let mut rb = self.find(b);
        if ra == rb { return; }
        let (ra_rank, rb_rank) = (self.rank[ra], self.rank[rb]);
        if ra_rank < rb_rank { std::mem::swap(&mut ra, &mut rb); }
        self.parent[rb] = ra;
        self.size[ra] += self.size[rb];
        if ra_rank == rb_rank { self.rank[ra] += 1; }
    }
}

#[derive(Clone, Copy)]
struct Args<'a> {
    edges_path: &'a str,
    nodes_path: Option<&'a str>,
    skip_header: bool,
    directed: bool,
}

fn parse_args() -> Args<'static> {
    let mut edges_path = "";
    let mut nodes_path: Option<&'static str> = None;
    let mut skip_header = false;
    let mut directed = false;

    let it = env::args().skip(1).collect::<Vec<_>>();
    let mut i = 0;
    while i < it.len() {
        match it[i].as_str() {
            "--edges" => { i+=1; edges_path = Box::leak(it[i].clone().into_boxed_str()); }
            "--nodes" => { i+=1; nodes_path = Some(Box::leak(it[i].clone().into_boxed_str())); }
            "--skip-header" => { skip_header = true; }
            "--directed" => { directed = true; }
            "-h" | "--help" => {
                eprintln!("Usage: compcount --edges <file> [--nodes <file>] [--skip-header] [--directed]");
                std::process::exit(0);
            }
            x => { eprintln!("unknown arg: {x}"); std::process::exit(2); }
        }
        i += 1;
    }
    if edges_path.is_empty() {
        eprintln!("missing --edges <file>");
        std::process::exit(2);
    }
    Args { edges_path, nodes_path, skip_header, directed }
}

fn open_reader(p: &str) -> io::Result<BufReader<File>> {
    let f = File::open(p)?;
    Ok(BufReader::new(f))
}

fn tokenize(line: &str) -> Vec<&str> {
    line.split(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == '\u{0001}' /* ctrl-A */)
        .filter(|t| !t.is_empty() && !t.starts_with('#'))
        .collect()
}

fn main() -> io::Result<()> {
    let args = parse_args();

    let mut dsu = Dsu::new();
    let mut id_map: HashMap<String, usize> = HashMap::new();
    let mut deg: HashMap<usize, u64> = HashMap::new();

    let ensure_node = |label: &str, dsu: &mut Dsu, id_map: &mut HashMap<String, usize>| -> usize {
        if let Some(&id) = id_map.get(label) { return id; }
        let id = dsu.add_node();
        id_map.insert(label.to_string(), id);
        id
    };

    // Preload nodes (optional)
    if let Some(np) = args.nodes_path {
        if Path::new(np).exists() {
            let rdr = open_reader(np)?;
            for line in rdr.lines() {
                let l = line?;
                let t = tokenize(&l);
                if t.is_empty() { continue; }
                let label = t[0];
                let _ = ensure_node(label, &mut dsu, &mut id_map);
            }
        }
    }

    // Read edges
    let mut rdr = open_reader(args.edges_path)?;
    let mut line = String::new();
    let mut first = true;
    let mut n_edges: u64 = 0;

    while {
        line.clear();
        rdr.read_line(&mut line)?
    } != 0 {
        if first && args.skip_header { first = false; continue; }
        first = false;

        let t = tokenize(&line);
        if t.len() < 2 { continue; }
        let u_lab = t[0];
        let v_lab = t[1];
        if u_lab == v_lab { // self-loop contributes degree but not components
            let u = ensure_node(u_lab, &mut dsu, &mut id_map);
            *deg.entry(u).or_insert(0) += 1;
            n_edges += 1;
            continue;
        }
        let u = ensure_node(u_lab, &mut dsu, &mut id_map);
        let v = ensure_node(v_lab, &mut dsu, &mut id_map);
        dsu.union(u, v);
        *deg.entry(u).or_insert(0) += 1;
        if !args.directed { *deg.entry(v).or_insert(0) += 1; }
        n_edges += 1;
    }

    // Compute component sizes
    let n_nodes = dsu.parent.len();
    let mut comp_size: HashMap<usize, usize> = HashMap::new();
    let mut comp_edges: HashMap<usize, u64> = HashMap::new();
    for i in 0..n_nodes {
        let r = dsu.find(i);
        *comp_size.entry(r).or_insert(0) += 1;
        let d = *deg.get(&i).unwrap_or(&0);
        *comp_edges.entry(r).or_insert(0) += d;
    }
    let mut sizes: Vec<usize> = comp_size.values().cloned().collect();
    sizes.sort_by(|a,b| b.cmp(a));

    let n_components = sizes.len();
    let largest = sizes.first().cloned().unwrap_or(0);
    let isolated = deg.iter().filter(|(_, &d)| d == 0).count();

    // Output JSON (manual)
    println!("{{");
    println!("  \"n_nodes\": {n_nodes},");
    println!("  \"n_edges\": {n_edges},");
    println!("  \"n_components\": {n_components},");
    println!("  \"largest_component\": {largest},");
    print!("  \"component_sizes\": [");
    for (i, s) in sizes.iter().enumerate() {
        if i > 0 { print!(", "); }
        print!("{s}");
    }
    println!("],");
    println!("  \"isolated_nodes\": {isolated}");
    println!("}}");

    Ok(())
}
