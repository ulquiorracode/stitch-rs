//! Architectural DAG extraction and visualization engine implementing `stitch graph`.

use serde::{Deserialize, Serialize};
use std::path::Path;
use syn::visit::Visit;
use syn::{ItemStruct, ItemTrait};
use walkdir::WalkDir;

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
}

impl<'a> GraphExtractor<'a> {
    pub fn new(root_dir: &'a Path) -> Self {
        Self { root_dir }
    }

    pub fn extract(&self) -> ArchitectureGraph {
        let mut graph = ArchitectureGraph::default();

        let rust_files = WalkDir::new(self.root_dir)
            .into_iter()
            .filter_entry(|entry| {
                let name = entry.file_name().to_string_lossy();
                let path_str = entry.path().to_string_lossy();
                entry.depth() == 0
                    || (name != "target"
                        && name != ".git"
                        && !name.starts_with('.')
                        && !path_str.contains("tests/ui")
                        && !path_str.contains("tests\\ui"))
            })
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"));

        for entry in rust_files {
            let path = entry.path();
            if let Ok(content) = std::fs::read_to_string(path)
                && let Ok(syntax_tree) = syn::parse_file(&content)
            {
                let mut visitor = NodeVisitor {
                    file_path: path.to_string_lossy().to_string(),
                    graph: &mut graph,
                };
                visitor.visit_file(&syntax_tree);
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
