use std::iter::repeat;

use factory_lang::{
    dfa_parser::{BuilderEdge, BuilderGraph, Dfa, Lookahead, dfa_to_dot},
    pattern_parser::parse_pattern,
    toml_parser::load,
};
use petgraph::graph::DiGraph;

fn main() {
    let pattern = parse_pattern("[{reg:register} + {offset:S6}]").unwrap();
    println!("Pattern parts: {:?}", pattern);

    let loaded_toml = load("eee");
    println!("Loaded toml: {:#?}", loaded_toml);

    let test_graph = {
        let mut graph = DiGraph::new();
        let start = graph.add_node(false);
        let accept = graph.add_node(true);

        // Rules
        // S -> "[" rawNumber "]"
        // S -> "[" rawNumber "i" "]"

        let n1 = add_chain(
            &mut graph,
            start,
            repeat(false).zip([
                literal("["),
                BuilderEdge::Lookahead(Lookahead::RawNumber { signed: false }),
                literal("]"),
            ]),
        );
        graph.add_edge(n1, accept, BuilderEdge::Lookahead(Lookahead::Always));

        let n2 = add_chain(
            &mut graph,
            start,
            repeat(false).zip([
                literal("["),
                BuilderEdge::Lookahead(Lookahead::RawNumber { signed: false }),
                literal("i"),
                literal("]"),
            ]),
        );
        graph.add_edge(n2, accept, BuilderEdge::Lookahead(Lookahead::Always));

        BuilderGraph {
            graph,
            start_node: start,
        }
    };

    let dfa = Dfa::from_builder_graph(&test_graph).unwrap();

    dfa_to_dot(&dfa, "dfa.dot");
}

fn literal(l: &str) -> BuilderEdge {
    BuilderEdge::Lookahead(Lookahead::Literal(l.into()))
}

fn add_chain<G: petgraph::data::Build>(
    g: &mut G,
    start: G::NodeId,
    nodes_and_edges: impl IntoIterator<Item = (G::NodeWeight, G::EdgeWeight)>,
) -> G::NodeId {
    let mut prev_node = start;
    for (n, e) in nodes_and_edges {
        let idx = g.add_node(n);
        g.add_edge(prev_node, idx, e);
        prev_node = idx;
    }
    prev_node
}
