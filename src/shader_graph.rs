use std::{
    collections::{HashMap, VecDeque},
    fmt::Display,
    io::BufRead,
    path::{Path, PathBuf},
    rc::Rc,
};

use crate::shader::{Shader, ShaderError, ShaderErrorVariant};

pub struct ShaderGraph {
    nodes: HashMap<PathBuf, Rc<ShaderGraphNode>>,
}

pub struct ShaderGraphNode {
    deps: Vec<Rc<ShaderGraphNode>>,
    code: String,
}

impl ShaderGraph {
    fn try_add_node(&mut self, path: &Path) -> Result<Rc<ShaderGraphNode>, ShaderError> {
        let canon_path = path.canonicalize()?;
        let file = std::fs::File::open(path)?;
        let mut reader = std::io::BufReader::new(file);

        let mut deps = Vec::new();
        let mut code = String::new();

        let mut line = String::new();
        let mut line_number = 0;
        while reader.read_line(&mut line)? != 0 {
            if !line.starts_with("//%") {
                code.push_str(line.as_str());
                line.clear();
                continue;
            }

            let parts = line.split_whitespace().skip(1).collect::<Vec<_>>();
            if parts.is_empty() {
                line.clear();
                continue;
            }

            let err =
                |variant, msg: &dyn Display| Err((format!("{line_number}: {msg}"), variant).into());

            match parts[0] {
                "include" => {
                    if parts.len() != 2 {
                        println!("[ERROR] {path:?}: {parts:?}");
                        return err(
                            ShaderErrorVariant::PPD,
                            &"include directive takes exactly one path argument",
                        );
                    }

                    // --- Path resolution
                    let sys_workdir = std::env::current_dir();
                    let workdir = path.parent().unwrap_or(
                        sys_workdir
                            .as_ref()
                            .expect("Should be valid working directory")
                            .as_path(),
                    );

                    let mut provided_path = parts[1][1..(parts[1].len() - 1)].to_string();
                    // Add .wgsl extension if it was omitted
                    if provided_path.len() < 5
                        || &provided_path[provided_path.len() - 5..] != ".wgsl"
                    {
                        provided_path.push_str(".wgsl");
                    }
                    let include_path = workdir.join(provided_path);
                    let include_path = include_path.canonicalize().expect(
                        format!(
                            "Provided path should be canonicalizable: `{}`",
                            include_path.to_str().unwrap()
                        )
                        .as_str(),
                    );

                    if let Some(node) = self.nodes.get(include_path.as_path()) {
                        deps.push(node.clone());
                        line.clear();
                        continue;
                    }

                    let include_node = std::fs::read_to_string(include_path.as_path())
                        .map_err(ShaderError::from)
                        .and_then(|code| self.try_add_node(include_path.as_path()))?;

                    deps.push(include_node);
                }
                _ => {
                    return err(
                        ShaderErrorVariant::PPD,
                        &format!("Unrecognized preprocessor directive: `{:?}`", parts[0]),
                    )
                }
            }

            line.clear();
            line_number += 1;
        }

        let node = Rc::new(ShaderGraphNode { deps, code });
        self.nodes.insert(canon_path, node.clone());

        Ok(node)
    }

    pub fn try_from_final(path: &Path) -> Result<Self, ShaderError> {
        let mut graph = Self {
            nodes: HashMap::new(),
        };

        graph.try_add_node(path)?;

        Ok(graph)
    }

    fn finish_dfs<'n>(
        &self,
        node: &'n Rc<ShaderGraphNode>,
        visited: &mut Vec<&'n Rc<ShaderGraphNode>>,
        target_buf: &mut String,
    ) {
        visited.push(node);

        for dep in node.deps.iter() {
            if !visited.iter().any(|n| Rc::ptr_eq(dep, n)) {
                self.finish_dfs(dep, visited, target_buf);
            }
        }

        target_buf.push_str(node.code.as_str());
    }

    pub fn finish(&self) -> Result<String, ShaderError> {
        // Find last node, i.e. the only node without any dependent
        let mut last = None;
        for node in self.nodes.values() {
            if !self
                .nodes
                .values()
                .any(|n| n.deps.iter().any(|dep| Rc::ptr_eq(dep, node)))
            {
                last = Some(node);
            }
        }

        let Some(last) = last else {
            return Err(ShaderError {
                msg: Some("Final file not found (maybe an include loop?)".to_owned()),
                variant: ShaderErrorVariant::PPD,
            });
        };

        let mut shader = String::new();
        self.finish_dfs(last, &mut vec![], &mut shader);
        Ok(shader)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use indoc::indoc;

    #[test]
    fn shader_graph() {
        // Setup
        let main = dbg!(indoc! {/*wgsl*/ r#"
            //% include "bar"
            //% include "foo"

            fn main() {}
        "#});

        let foo = dbg!(indoc! {/*wgsl*/ r#"
            //% include "bar"
            
            fn foo() {}
        "#});

        let bar = dbg!(indoc! {/*wgsl*/ r#"
            fn bar() {}
        "#});

        if !Path::new(".test_dir").is_dir() {
            std::fs::create_dir(".test_dir").expect(".test_dir/ should be successfully created");
        }

        std::env::set_current_dir(".test_dir");

        std::fs::write("main.wgsl", main)
            .and_then(|_| std::fs::write("foo.wgsl", foo))
            .and_then(|_| std::fs::write("bar.wgsl", bar))
            .expect("Wgsl test files should be written to .test_dir");

        // Logic
        let graph = ShaderGraph::try_from_final(Path::new("main.wgsl"))
            .expect("Graph should be properly created");

        let code = graph
            .finish()
            .expect("Final code should be created properly");

        assert_eq!(
            code.trim(),
            indoc! {r#"
                fn bar() {}
                
                fn foo() {}

                fn main() {}
            "#}
            .trim()
        );

        // Cleanup
        std::fs::remove_file("main.wgsl")
            .and_then(|_| std::fs::remove_file("foo.wgsl"))
            .and_then(|_| std::fs::remove_file("bar.wgsl"))
            .expect("Wgsl test files should be deleted");
    }
}
