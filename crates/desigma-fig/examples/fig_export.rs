//! Renders PNGs and exports SVGs of `.fig` nodes addressed by their REST ids
//! (the ids `fig2json --rest` writes), for tools that pull a file node by node.
//!
//!   cargo run --release -p desigma-fig --example fig_export -- file.fig jobs.json
//!
//! `jobs.json`: `{"png": [{"id": "1:2", "scale": 2, "out": "a.png"}], "svg": [{"id": "I1:3;4:5", "out": "b.svg"}]}`.
//! Prints `{"done": n, "missing": [ids], "empty": [ids]}`: ids not in the file, and nodes that drew nothing.

use desigma_doc::NodeId;
use desigma_export::{export_png, export_svg, SvgOptions};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [fig, jobs] = args.as_slice() else {
        eprintln!("usage: fig_export <file.fig> <jobs.json>");
        std::process::exit(2);
    };
    let bytes = std::fs::read(fig).unwrap_or_else(|e| fail(&format!("cannot read {fig}: {e}")));
    let doc = desigma_fig::import_fig(&bytes).unwrap_or_else(|e| fail(&format!("import failed: {e}")));
    let jobs: Value = serde_json::from_str(&std::fs::read_to_string(jobs).unwrap_or_else(|e| fail(&format!("cannot read {jobs}: {e}"))))
        .unwrap_or_else(|e| fail(&format!("bad jobs json: {e}")));
    let by_rest: HashMap<&str, NodeId> = doc.nodes.values().filter_map(|n| Some((n.source_id.as_deref()?, n.id))).collect();

    let (mut done, mut missing, mut empty) = (0usize, Vec::new(), Vec::new());
    let mut each = |kind: &str, run: &mut dyn FnMut(NodeId, &Value) -> Vec<u8>| {
        for j in jobs.get(kind).and_then(Value::as_array).into_iter().flatten() {
            let (Some(id), Some(out)) = (j.get("id").and_then(Value::as_str), j.get("out").and_then(Value::as_str)) else { continue };
            let Some(&node) = by_rest.get(id) else {
                missing.push(id.to_string());
                continue;
            };
            let data = run(node, j);
            if data.is_empty() {
                empty.push(id.to_string());
                continue;
            }
            if let Some(dir) = Path::new(out).parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            std::fs::write(out, data).unwrap_or_else(|e| fail(&format!("cannot write {out}: {e}")));
            done += 1;
        }
    };
    each("png", &mut |node, j| export_png(&doc, node, j.get("scale").and_then(Value::as_f64).unwrap_or(1.0) as f32));
    each("svg", &mut |node, _| export_svg(&doc, node, &SvgOptions::default()).into_bytes());
    println!("{}", json!({"done": done, "missing": missing, "empty": empty}));
}

fn fail(msg: &str) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}
