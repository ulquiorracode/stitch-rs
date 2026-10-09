use crate::config::{ScopeFilter, collect_rust_files};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use syn::visit::Visit;
use syn::{ItemImpl, ItemStruct, ItemTrait};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureGraph {
    pub schema: String,
    pub version: String,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub pipelines: Vec<PipelineModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub name: String,
    pub role: String,
    pub file: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub kind: String, // "Implements", "Requires", "Traverses"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineModel {
    pub name: String,
    pub blackboard: String,
    pub layers: Vec<String>,
    pub terminal: String,
}

impl Default for ArchitectureGraph {
    fn default() -> Self {
        Self {
            schema: "https://goldsrc.rs/schemas/sma-graph-v1.json".to_string(),
            version: "1.0.0".to_string(),
            nodes: Vec::new(),
            edges: Vec::new(),
            pipelines: Vec::new(),
        }
    }
}

pub struct GraphExtractor<'a> {
    root_dir: &'a Path,
    scope: Option<ScopeFilter>,
}

impl<'a> GraphExtractor<'a> {
    pub fn new(root_dir: &'a Path) -> Self {
        Self {
            root_dir,
            scope: None,
        }
    }

    pub fn new_scoped(root_dir: &'a Path, scope: Option<ScopeFilter>) -> Self {
        Self { root_dir, scope }
    }

    pub fn extract(&self) -> ArchitectureGraph {
        let mut graph = ArchitectureGraph::default();

        let rust_files = collect_rust_files(self.root_dir, self.scope.as_ref());

        for path in rust_files {
            if let Ok(content) = std::fs::read_to_string(&path)
                && let Ok(syntax_tree) = syn::parse_file(&content)
            {
                let mut visitor = NodeVisitor {
                    file_path: path.to_string_lossy().to_string(),
                    graph: &mut graph,
                };
                visitor.visit_file(&syntax_tree);
            }
        }

        // Apply scope filter with 1-hop boundary context preservation
        if let Some(scope) = &self.scope
            && !scope.is_empty()
        {
            let direct_matches: HashSet<String> = graph
                .nodes
                .iter()
                .filter(|n| scope.matches_path(Path::new(&n.file)) || scope.matches_name(&n.name))
                .map(|n| n.id.clone())
                .collect();

            if !direct_matches.is_empty() {
                let mut retained_ids = direct_matches.clone();
                for edge in &graph.edges {
                    if direct_matches.contains(&edge.source) {
                        retained_ids.insert(edge.target.clone());
                    }
                    if direct_matches.contains(&edge.target) {
                        retained_ids.insert(edge.source.clone());
                    }
                }
                graph.nodes.retain(|n| retained_ids.contains(&n.id));
                graph.edges.retain(|e| {
                    retained_ids.contains(&e.source) && retained_ids.contains(&e.target)
                });
            }
        }

        graph
    }
}

struct NodeVisitor<'a> {
    file_path: String,
    graph: &'a mut ArchitectureGraph,
}

impl<'a, 'ast> Visit<'ast> for NodeVisitor<'a> {
    fn visit_item_struct(&mut self, i: &'ast ItemStruct) {
        let name = i.ident.to_string();
        let role = if name.ends_with("Layer") {
            "Layer"
        } else if name.ends_with("Terminal") {
            "Terminal"
        } else if name.ends_with("Adapter") {
            "Adapter"
        } else if name.ends_with("Hub") {
            "Hub"
        } else if name.ends_with("Context") || name.ends_with("Blackboard") {
            "Blackboard"
        } else if name.ends_with("Token") {
            "Token"
        } else if name.ends_with("Id") {
            "Id"
        } else {
            ""
        };

        if !role.is_empty() {
            let line = i.ident.span().start().line;
            self.graph.nodes.push(GraphNode {
                id: name.clone(),
                name,
                role: role.to_string(),
                file: self.file_path.clone(),
                line,
            });
        }
        syn::visit::visit_item_struct(self, i);
    }

    fn visit_item_trait(&mut self, i: &'ast ItemTrait) {
        let name = i.ident.to_string();
        if name.ends_with("Port") {
            let line = i.ident.span().start().line;
            self.graph.nodes.push(GraphNode {
                id: name.clone(),
                name,
                role: "Port".to_string(),
                file: self.file_path.clone(),
                line,
            });
        }
        syn::visit::visit_item_trait(self, i);
    }

    fn visit_item_impl(&mut self, i: &'ast ItemImpl) {
        if let Some((_, trait_path, _)) = &i.trait_ {
            let trait_name = trait_path
                .segments
                .last()
                .map(|s| s.ident.to_string())
                .unwrap_or_default();
            let self_ty = &i.self_ty;
            let self_type_str = quote::quote!(#self_ty).to_string();
            let self_clean = self_type_str
                .split('<')
                .next()
                .unwrap_or(&self_type_str)
                .trim()
                .to_string();

            if trait_name.ends_with("Port") {
                self.graph.edges.push(GraphEdge {
                    source: self_clean,
                    target: trait_name,
                    kind: "Implements".to_string(),
                });
            }
        }
        syn::visit::visit_item_impl(self, i);
    }
}

impl ArchitectureGraph {
    /// Renders Mermaid diagram string.
    pub fn to_mermaid(&self) -> String {
        let mut out = String::from("```mermaid\nflowchart TD\n");
        out.push_str("    subgraph CoreDomain [Core & SPI]\n");
        for n in &self.nodes {
            if n.role == "Port" {
                out.push_str(&format!(
                    "        {node_id}[\"Port: {name}\"]\n",
                    node_id = n.id,
                    name = n.name
                ));
            }
        }
        out.push_str("    end\n\n");

        out.push_str("    subgraph Adapters [Vendor & Engine Adapters]\n");
        for n in &self.nodes {
            if n.role == "Adapter" {
                out.push_str(&format!(
                    "        {node_id}[\"Adapter: {name}\"]\n",
                    node_id = n.id,
                    name = n.name
                ));
            }
        }
        out.push_str("    end\n\n");

        out.push_str("    subgraph PipelineNodes [SMA U-Cycle Nodes]\n");
        for n in &self.nodes {
            if n.role == "Layer" || n.role == "Terminal" || n.role == "Blackboard" {
                out.push_str(&format!(
                    "        {node_id}[\"{role}: {name}\"]\n",
                    node_id = n.id,
                    role = n.role,
                    name = n.name
                ));
            }
        }
        out.push_str("    end\n");

        if !self.edges.is_empty() {
            out.push_str("\n    %% Architectural Boundaries & Implementations\n");
            for e in &self.edges {
                out.push_str(&format!(
                    "    {} -. \"{}\" .-> {}\n",
                    e.source, e.kind, e.target
                ));
            }
        }

        out.push_str("```\n");
        out
    }

    /// Renders Graphviz DOT format.
    pub fn to_dot(&self) -> String {
        let mut out = String::from(
            "digraph SMA_Architecture {\n    node [shape=box, fontname=\"Helvetica\"];\n",
        );
        for n in &self.nodes {
            out.push_str(&format!(
                "    \"{}\" [label=\"{}\\n<{}>\"];\n",
                n.id, n.name, n.role
            ));
        }
        for e in &self.edges {
            out.push_str(&format!(
                "    \"{}\" -> \"{}\" [label=\"{}\", style=dashed];\n",
                e.source, e.target, e.kind
            ));
        }
        out.push_str("}\n");
        out
    }

    /// Renders standalone HTML page with embedded interactive SVG/Mermaid.
    pub fn to_html(&self) -> String {
        let mermaid_code = self
            .to_mermaid()
            .replace("```mermaid\n", "")
            .replace("```\n", "");
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>SMA Architecture Graph</title>
    <script type="module">
        import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.esm.min.mjs';
        mermaid.initialize({{ startOnLoad: true, theme: 'dark' }});
    </script>
    <style>
        body {{ background-color: #1a1a1a; color: #eee; font-family: sans-serif; padding: 20px; }}
        h1 {{ color: #61afef; }}
    </style>
</head>
<body>
    <h1>Sewing Machine Architecture (SMA) DAG</h1>
    <pre class="mermaid">
{mermaid_code}
    </pre>
</body>
</html>
"#
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_graph_extract_and_renders() {
        let temp_dir =
            std::env::temp_dir().join(format!("stitch_graph_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).expect("create temp dir");

        let code = r#"
pub trait LogPort {
    fn log(&self, msg: &str);
}

pub struct StdoutAdapter;

impl LogPort for StdoutAdapter {
    fn log(&self, msg: &str) {}
}

pub struct AuthLayer;
pub struct ExecTerminal;
pub struct PacketBlackboard;
"#;
        fs::write(temp_dir.join("domain.rs"), code).expect("write temp source");

        let extractor = GraphExtractor::new(&temp_dir);
        let graph = extractor.extract();

        let node_names: Vec<&str> = graph.nodes.iter().map(|n| n.name.as_str()).collect();
        assert!(node_names.contains(&"LogPort"));
        assert!(node_names.contains(&"StdoutAdapter"));
        assert!(node_names.contains(&"AuthLayer"));
        assert!(node_names.contains(&"ExecTerminal"));
        assert!(node_names.contains(&"PacketBlackboard"));

        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].source, "StdoutAdapter");
        assert_eq!(graph.edges[0].target, "LogPort");
        assert_eq!(graph.edges[0].kind, "Implements");

        let mermaid = graph.to_mermaid();
        assert!(mermaid.contains("subgraph CoreDomain"));
        assert!(mermaid.contains("LogPort"));
        assert!(mermaid.contains("StdoutAdapter"));
        assert!(mermaid.contains("AuthLayer"));

        let dot = graph.to_dot();
        assert!(dot.contains("digraph SMA_Architecture"));
        assert!(dot.contains("\"StdoutAdapter\" -> \"LogPort\""));

        let html = graph.to_html();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("flowchart TD"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_graph_scoped_filter_one_hop() {
        let temp_dir =
            std::env::temp_dir().join(format!("stitch_graph_scope_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).expect("create temp dir");

        let code = r#"
pub trait StoragePort {}
pub struct DiskAdapter;
impl StoragePort for DiskAdapter {}

pub struct UnrelatedLayer;
"#;
        fs::write(temp_dir.join("storage.rs"), code).expect("write temp source");

        let scope = ScopeFilter::new("DiskAdapter");

        let extractor = GraphExtractor::new_scoped(&temp_dir, Some(scope));
        let graph = extractor.extract();

        let node_names: Vec<&str> = graph.nodes.iter().map(|n| n.name.as_str()).collect();
        assert!(node_names.contains(&"DiskAdapter"));
        // 1-hop boundary context retains StoragePort because DiskAdapter implements StoragePort
        assert!(node_names.contains(&"StoragePort"));
        // UnrelatedLayer is filtered out
        assert!(!node_names.contains(&"UnrelatedLayer"));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
