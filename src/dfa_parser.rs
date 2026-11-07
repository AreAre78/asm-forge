use std::{collections::HashMap, ops::Neg, rc::Rc};

use ibig::{IBig, error::ParseError};
use petgraph::Direction;
use petgraph::dot::Config::NodeNoLabel;
use petgraph::dot::Dot;
use petgraph::graph::DiGraph;
use petgraph::{graph::NodeIndex, visit::EdgeRef};
use thiserror::Error;
use trivet::{Parser, parse_from_string};

// --- DFA ---

/// The start state should be at index 0
#[derive(Debug, Clone)]
pub struct Dfa {
    nodes: Vec<DfaNode>,
    start_node: usize,
}

type DfaNodeId = usize;

#[derive(Debug, Clone)]
struct DfaNode {
    edges: Vec<DfaEdge>,
    is_accept_state: bool,
}

#[derive(Debug, Clone)]
struct DfaEdge {
    lookahead: Lookahead,
    next_node: DfaNodeId,
    reduce_fn: Vec<()>,
}

// TODO: Add consumes_trailing_whitespace
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Lookahead {
    Literal(Rc<str>),
    Number,
    RawNumber {
        signed: bool,
    },
    Eof,
    /// Always matches last
    Always,
}

impl Lookahead {
    fn rank(&self) -> u32 {
        match self {
            Lookahead::Literal(_) => 0,
            Lookahead::Number => 1,
            Lookahead::RawNumber { .. } => 1,
            Lookahead::Eof => 2,
            Lookahead::Always => 3,
        }
    }
}

// The number regex is /^-?(((0d)?[0-9_]+)|(0x[0-9a-f_]+)|(0b[01_]+)|(0o[0-7_]+))/i

// --- Parser ---

pub fn parse(dfa: &Dfa, instruction: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = parse_from_string(instruction);

    let mut node_id = dfa.start_node;
    let mut stack = Vec::new();

    'outer: loop {
        let node = &dfa.nodes[node_id];
        if node.is_accept_state {
            break;
        }

        for edge in &node.edges {
            match try_parse_lookahead(&mut parser, &edge.lookahead)? {
                LookaheadResult::NoMatch => continue,
                LookaheadResult::Match => {}
                LookaheadResult::MatchWithData(num) => {
                    stack.push(num);
                }
            };
            node_id = edge.next_node;
            for _f in &edge.reduce_fn {
                unimplemented!();
            }
            continue 'outer;
        }

        todo!("Implement the parse fail case")
    }

    Ok(())
}

enum LookaheadResult {
    NoMatch,
    Match,
    MatchWithData(IBig),
}

fn try_parse_lookahead(
    parser: &mut Parser,
    lookahead: &Lookahead,
) -> Result<LookaheadResult, NoNumberAfterRadixOrSign> {
    Ok(match lookahead {
        Lookahead::Literal(literal) => {
            if parser.peek_and_consume_str(literal) {
                LookaheadResult::Match
            } else {
                LookaheadResult::NoMatch
            }
        }
        Lookahead::Number => todo!(),
        Lookahead::RawNumber { signed } => match parse_raw_number(parser, *signed, false, false)? {
            Some(num) => LookaheadResult::MatchWithData(num),
            None => LookaheadResult::NoMatch,
        },
        Lookahead::Eof => {
            if parser.is_at_eof() {
                LookaheadResult::Match
            } else {
                LookaheadResult::NoMatch
            }
        }
        Lookahead::Always => LookaheadResult::Match,
    })
}

#[derive(Debug, Error)]
#[error("There were no numbers after the radix or sign")]
struct NoNumberAfterRadixOrSign;

fn parse_raw_number(
    parser: &mut Parser,
    signed: bool,
    allow_custom_base: bool,
    allow_underscore: bool,
) -> Result<Option<IBig>, NoNumberAfterRadixOrSign> {
    let initial_pos = parser.loc();
    let negative = signed && parser.peek_and_consume('-');

    let base10 = (10, char::is_ascii_digit as fn(&char) -> bool);
    let (base, take_fn): (_, fn(&char) -> bool) = if allow_custom_base && parser.peek() == '0' {
        match parser.peek_offset(1) {
            'x' | 'X' => {
                parser.consume_n(2);
                (16, char::is_ascii_hexdigit)
            }
            'b' | 'B' => {
                parser.consume_n(2);
                (2, |c: &char| *c == '0' || *c == '1')
            }
            'o' | 'O' => {
                parser.consume_n(2);
                (8, char::is_ascii_octdigit)
            }
            'd' | 'D' => {
                parser.consume_n(2);
                base10
            }
            _ => base10,
        }
    } else {
        base10
    };

    let raw_digits = parser.take_while_unless(|c| take_fn(&c), |c| allow_underscore && c == '_');

    Ok(match IBig::from_str_radix(&raw_digits, base) {
        Ok(num) => Some(num),
        Err(ParseError::NoDigits) => {
            // If the parser has consumed anything it would be unsound to continue using it
            if parser.loc() == initial_pos {
                None
            } else {
                return Err(NoNumberAfterRadixOrSign);
            }
        }
        Err(ParseError::InvalidDigit) => unreachable!("Only valid digits should be collected"),
    }
    .map(|n| if negative { n.neg() } else { n }))
}

// --- Builder ---

#[derive(Debug, Clone)]
pub struct BuilderGraph {
    pub graph: DiGraph<bool, BuilderEdge>,
    pub start_node: NodeIndex,
}

#[derive(Debug, Clone)]
pub struct InlinedBuilderGraph {
    graph: DiGraph<bool, Lookahead>,
    /// The start now cannot be accepting.
    /// Use `Lookahead::Empty` for that
    start_node: NodeIndex,
}

// TODO: Add reduce functions
#[derive(Debug, Clone)]
pub enum BuilderEdge {
    Lookahead(Lookahead),
    /// The start now cannot be accepting.
    /// Use `Lookahead::Empty` for that
    Inner(Rc<InlinedBuilderGraph>),
}

impl Dfa {
    pub fn from_builder_graph(graph: &BuilderGraph) -> Result<Self, Box<dyn std::error::Error>> {
        let mut inlined_graph = inline_builder_graph(graph);
        front_merge(&mut inlined_graph)?;
        Ok(sort_to_dfa_graph(&inlined_graph))
    }
}

// steps:
// [x] inline
// [-] resolve empty lookahead to non accepting nodes
// [x] merge
// [-] optimization/cleanup
// [x] sort

fn inline_builder_graph(graph: &BuilderGraph) -> InlinedBuilderGraph {
    let mut to_be_inlined = Vec::new();

    let mut new_graph = graph.graph.filter_map(
        |_, &a| Some(a),
        |idx, e| {
            if let BuilderEdge::Lookahead(l) = e {
                Some(l.clone())
            } else {
                to_be_inlined.push(idx);
                None
            }
        },
    );

    for idx in to_be_inlined {
        let BuilderEdge::Inner(to_inline) = graph.graph.edge_weight(idx).unwrap() else {
            panic!("All edges in to_be_inlined should be BuilderEdge::Inner")
        };
        let (a, b) = graph.graph.edge_endpoints(idx).unwrap();
        inline_graph(&mut new_graph, to_inline, a, b);
    }

    new_graph.shrink_to_fit();

    InlinedBuilderGraph {
        graph: new_graph,
        start_node: graph.start_node,
    }
}

fn inline_graph(
    g: &mut DiGraph<bool, Lookahead>,
    h: &InlinedBuilderGraph,
    start: NodeIndex,
    end: NodeIndex,
) {
    let mut node_map = HashMap::new();

    for idx in h.graph.node_indices() {
        let accept = *h.graph.node_weight(idx).unwrap();

        node_map.insert(idx, {
            assert!(
                !(idx == h.start_node && accept),
                "Start node cannot be accept node"
            );
            if idx == h.start_node {
                start
            } else if accept {
                end
            } else {
                g.add_node(false)
            }
        });
    }

    for edge in h.graph.edge_references() {
        g.add_edge(
            node_map[&edge.source()],
            node_map[&edge.target()],
            edge.weight().clone(),
        );
    }
}

// Need cleanup
fn front_merge(g: &mut InlinedBuilderGraph) -> Result<(), Box<dyn std::error::Error>> {
    while let Some((idx, other_idx)) = g.graph.node_indices().find_map(|idx| {
        let mut hashmap = HashMap::new();
        for edge in g.graph.edges(idx) {
            if let Some(other_idx) = hashmap.insert(edge.weight(), edge.id()) {
                // TODO: Check for different reduction functions
                return Some((edge.id(), other_idx));
            };
        }
        None
    }) {
        let src1 = g.graph.edge_endpoints(idx).unwrap().1;
        let src2 = g.graph.edge_endpoints(other_idx).unwrap().1;
        let dst = g.graph.add_node(g.graph[src1] || g.graph[src2]);
        g.graph.add_edge(
            g.graph.edge_endpoints(idx).unwrap().0,
            dst,
            g.graph[idx].clone(),
        );
        copy_edges(&mut g.graph, dst, src1, Direction::Outgoing)?;
        copy_edges(&mut g.graph, dst, src2, Direction::Outgoing)?;
        g.graph.remove_edge(idx);
        g.graph.remove_edge(other_idx);
    }
    Ok(())
}

// TODO: Move somewhere else and rename
#[derive(Debug, Error)]
#[error("There is a ambiguity in whether to reduce or push")]
struct ReduceAmbiguity;

// Need cleanup
fn copy_edges(
    graph: &mut DiGraph<bool, Lookahead>,
    destination: NodeIndex,
    source: NodeIndex,
    direction: Direction,
) -> Result<(), ReduceAmbiguity> {
    let edges = graph
        .edges_directed(source, direction)
        .map(|e| (e.target(), e.weight().clone()))
        .collect::<Vec<_>>();
    for (edge_target, weight) in edges {
        let (edge_source, edge_target) = if direction == Direction::Outgoing {
            (destination, edge_target)
        } else {
            (edge_target, destination)
        };

        let already_exists = graph
            .edges_connecting(edge_source, edge_target)
            .try_find(|other_edge| Ok(other_edge.weight() == &weight))?
            .is_some();

        if !already_exists {
            graph.add_edge(edge_source, edge_target, weight);
        }
    }
    Ok(())
}

fn sort_to_dfa_graph(g: &InlinedBuilderGraph) -> Dfa {
    let mut node_mapping = HashMap::new();
    let mut nodes = Vec::new();

    for idx in g.graph.node_indices() {
        node_mapping.insert(idx, nodes.len());
        nodes.push(DfaNode {
            edges: Vec::new(),
            is_accept_state: g.graph[idx],
        });
    }

    for edge in g.graph.edge_references() {
        let lookahead = edge.weight().clone();
        let node_index = node_mapping[&edge.source()];
        let next_node = node_mapping[&edge.target()];
        nodes[node_index].edges.push(DfaEdge {
            lookahead,
            next_node,
            reduce_fn: Vec::new(),
        });
    }

    for node in &mut nodes {
        node.edges
            .sort_by(|a, b| match (&a.lookahead, &b.lookahead) {
                (Lookahead::Literal(l1), Lookahead::Literal(l2)) => l2.len().cmp(&l1.len()),
                (al, bl) => al.rank().cmp(&bl.rank()),
            });
    }

    Dfa {
        nodes,
        start_node: node_mapping[&g.start_node],
    }
}

// --- Visualize ---
// This part is partially written by AI

fn lookahead_to_label(lookahead: &Lookahead) -> String {
    match lookahead {
        Lookahead::Literal(literal) => literal.to_string(),
        Lookahead::Number => "number".to_string(),
        Lookahead::RawNumber { signed } => {
            format!(
                "raw_number({})",
                if *signed { "signed" } else { "unsigned" }
            )
        }
        Lookahead::Eof => "<EOF>".to_string(),
        Lookahead::Always => "<Always>".to_string(),
    }
}

pub fn dfa_to_dot(dfa: &Dfa, path: impl AsRef<std::path::Path>) {
    let mut graph = DiGraph::new();

    let node_ids: Vec<_> = dfa
        .nodes
        .iter()
        .map(|n| graph.add_node(n.is_accept_state))
        .collect();

    for (from_idx, node) in dfa.nodes.iter().enumerate() {
        let from_node = node_ids[from_idx];
        for (attempt_order, edge) in node.edges.iter().enumerate() {
            let to_node = node_ids[edge.next_node];
            let functions = edge
                .reduce_fn
                .iter()
                .map(|_f| "0")
                .collect::<Vec<_>>()
                .join(", ");
            graph.add_edge(
                from_node,
                to_node,
                format!(
                    "#{attempt_order}\n{}\n{functions}",
                    lookahead_to_label(&edge.lookahead)
                ),
            );
        }
    }

    let dot = Dot::with_attr_getters(
        &graph,
        &[NodeNoLabel],
        &|_, _| "color = gray".to_string(),
        &|_, (n, a)| {
            let color = if n == node_ids[dfa.start_node] {
                "green"
            } else if *a {
                "red"
            } else {
                "lightgray"
            };
            format!(r#"style = filled, fillcolor = "{color}""#).to_string()
        },
    )
    .to_string();
    std::fs::write(path, dot).unwrap();
}
