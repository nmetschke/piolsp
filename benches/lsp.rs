use criterion::{Criterion, criterion_group, criterion_main};
use gen_lsp_types::Position;
use piolsp::lsp::*;
use std::hint::black_box;

fn analyze<'a>(text: &str, doc: &'a mut DocumentData) -> Option<&'a str> {
    doc.analyze_programs(text, (0, 0));
    doc.programs.get().programs.iter().next().map(|e| *e.0)
}

fn criterion_benchmark(c: &mut Criterion) {
    let t =
        String::from_utf8(std::fs::read("./tests/pico-examples/pio/spi/spi.pio").unwrap()).unwrap();
    let mut p = piolsp::pio_parser();
    let mut d = DocumentData::new(&mut p, t.clone());

    c.bench_function("parse", |b| {
        b.iter(|| black_box(p.parse(black_box(&t), None)));
    });

    c.bench_function("DocumentData::new", |b| {
        b.iter(|| black_box(DocumentData::new(&mut p, black_box(t.clone()))));
    });
    c.bench_function("DocumentData::analyze", |b| {
        b.iter(|| {
            black_box(analyze(black_box(&t), black_box(&mut d)));
        });
    });
    c.bench_function("DocumentData::update", |b| {
        b.iter(|| d.update(&mut p, black_box(t.clone())));
    });

    let t = String::from_utf8(
        std::fs::read("./tests/pico-examples/pio/ir_nec/nec_receive_library/nec_receive.pio")
            .unwrap(),
    )
    .unwrap();
    let mut p = piolsp::pio_parser();
    let d = DocumentData::new(&mut p, t.clone());

    c.bench_function("DocumentData::find_definition", |b| {
        b.iter(|| {
            let pos = Position::new(41 - 1, 14 - 1);
            let node = d.node_at(pos).unwrap();
            let def = d.get_definition(node).unwrap();
            assert_eq!(def.text, "BIT_SAMPLE_DELAY");

            match def.typ {
                SymbolType::Define { value } => assert_eq!(value, "15"),
                _ => panic!("expected define"),
            }
        });
    });
    c.bench_function("DocumentData::get_references", |b| {
        b.iter(|| {
            let pos = Position::new(32 - 1, 16 - 1);
            let node = d.node_at(pos).unwrap();
            let (_, refs) = d.get_references(node).unwrap();
            assert_eq!(refs.len(), 1);
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
