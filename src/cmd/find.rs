//! find, by name only: -name, -type f/d and -maxdepth. Results come out
//! depth first with each folder's children in byte order.

use crate::engine::Runner;
use crate::glob::{self, Tok};
use crate::tree::{Db, NodeId};
use crate::util::{basename, join, lossy, quote};

#[derive(Default)]
struct Expr {
    names: Vec<Vec<Tok>>,
    types: Vec<u8>,
    maxdepth: Option<usize>,
}

pub fn find(args: Vec<Vec<u8>>) -> Result<Runner, String> {
    let split = args
        .iter()
        .position(|a| a.starts_with(b"-") && a.len() > 1 || matches!(a.as_slice(), b"(" | b")" | b"!" | b","))
        .unwrap_or(args.len());
    let mut starts: Vec<Vec<u8>> = args[..split].to_vec();
    if starts.is_empty() {
        starts.push(b".".to_vec());
    }
    let mut expr = Expr::default();
    // An error GNU find reports itself, after the flags have been accepted.
    let mut usage_error: Option<String> = None;
    let mut i = split;
    while i < args.len() {
        let tok = args[i].as_slice();
        if !matches!(tok, b"-name" | b"-type" | b"-maxdepth") {
            return Err(format!("flag: find {}", lossy(tok)));
        }
        let arg = match args.get(i + 1) {
            Some(a) => a.as_slice(),
            None => {
                usage_error.get_or_insert(format!("find: missing argument to `{}'", lossy(tok)));
                break;
            }
        };
        i += 2;
        match tok {
            b"-name" => expr.names.push(glob::compile(&glob::fnmatch_pattern(arg))),
            b"-type" => match arg {
                b"f" | b"d" => expr.types.push(arg[0]),
                [c] if b"bcpls".contains(c) => return Err(format!("flag: find -type {}", char::from(*c))),
                _ if arg.contains(&b',') => return Err("flag: find -type with a list of types".into()),
                _ => {
                    usage_error.get_or_insert(format!("find: Unknown argument to -type: {}", lossy(arg)));
                }
            },
            _ => {
                if !arg.is_empty() && arg.iter().all(|c| c.is_ascii_digit()) {
                    expr.maxdepth = Some(arg.iter().fold(0usize, |n, c| n.saturating_mul(10).saturating_add((c - b'0') as usize)));
                } else {
                    usage_error.get_or_insert(format!(
                        "find: Expected a positive decimal integer argument to -maxdepth, but got {}",
                        quote(arg)
                    ));
                }
            }
        }
    }
    Ok(Box::new(move |db, io| {
        if let Some(e) = usage_error {
            io.err_line(e);
            return 1;
        }
        let mut status = 0;
        let mut out = Vec::new();
        for start in &starts {
            match db.resolve(start) {
                Err(e) => {
                    io.err_line(format!("find: {}: {}", quote(start), e.msg()));
                    status = 1;
                }
                Ok(id) => visit(db, &expr, start, basename(start), id, 0, &mut out),
            }
        }
        io.out = out;
        status
    }))
}

fn visit(db: &Db, expr: &Expr, path: &[u8], name: &[u8], id: NodeId, depth: usize, out: &mut Vec<u8>) {
    let is_dir = db.is_dir(id);
    let kind = if is_dir { b'd' } else { b'f' };
    if expr.types.iter().all(|t| *t == kind) && expr.names.iter().all(|n| glob::matches(n, name)) {
        out.extend_from_slice(path);
        out.push(b'\n');
    }
    if is_dir && expr.maxdepth.map_or(true, |m| depth < m) {
        for (child_name, child) in db.children(id) {
            visit(db, expr, &join(path, child_name), child_name, *child, depth + 1, out);
        }
    }
}
