use std::{
    collections::VecDeque,
    fmt::{Display, Write},
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct Shader(VecDeque<ShaderEntry>);

type ShaderEntry = (Option<PathBuf>, String);

impl Shader {
    pub fn new() -> Self {
        Shader(VecDeque::new())
    }

    pub fn prepend(self, mut shader: Self) -> Self {
        shader.0.extend(self.into_inner());
        shader
    }

    pub fn append(mut self, shader: Self) -> Self {
        self.0.extend(shader.into_inner());
        self
    }

    pub fn finish(self) -> String {
        self.0
            .into_iter()
            .map(|(path, code)| {
                path.map(|path| format!("// {path:?}\n\n{code}"))
                    .unwrap_or(code)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn into_inner(self) -> VecDeque<ShaderEntry> {
        self.0
    }

    fn preprocess(path: &Path, code: String) -> Result<VecDeque<ShaderEntry>, ShaderError> {
        let mut vec: VecDeque<_> = vec![].into();

        // Apply pre-processing commands
        let code = code
            .lines()
            .enumerate()
            .flat_map(|(index, line)| -> Option<Result<&str, ShaderError>> {
                if !line.starts_with("//%") {
                    return Some(Ok(line));
                }

                let parts = line.split_whitespace().skip(1).collect::<Vec<_>>();
                if parts.is_empty() {
                    return None;
                }

                let err = |variant, msg: &dyn Display| {
                    Some(Err((format!("{index}: {msg}"), variant).into()))
                };

                match parts[0] {
                    "include" => {
                        if parts.len() != 2 {
                            return err(
                                ShaderErrorVariant::PPD,
                                &"include directive takes exactly one path argument",
                            );
                        }

                        let sys_workdir = std::env::current_dir()
                            .expect("System working directory should be valid");

                        let workdir = path
                            .parent()
                            .map(|p| sys_workdir.join(p))
                            .unwrap_or(sys_workdir);

                        let include_path = format!("{}.wgsl", &parts[1][1..(parts[1].len() - 1)]);
                        let include_path = workdir.join(include_path);
                        let include_path = dbg!(&include_path).canonicalize().expect(
                            format!(
                                "you shouldn't do weird stuff with shader paths :( ({})",
                                include_path.to_str().unwrap()
                            )
                            .as_str(),
                        );

                        let include = std::fs::read_to_string(include_path.as_path())
                            .map_err(ShaderError::from)
                            .and_then(|code| Self::preprocess(include_path.as_path(), code));

                        match include {
                            Ok(mut include_vec) => {
                                while let Some(entry) = include_vec.pop_back() {
                                    vec.push_front(entry);
                                }
                            }
                            Err(e) => return Some(Err(e)),
                        }
                    }
                    _ => (),
                }

                None
            })
            .collect::<Result<Vec<_>, _>>()?
            .join("\n");

        vec.push_back((Some(path.into()), code));

        Ok(vec)
    }
}

impl<'i> TryFrom<&'i Path> for Shader {
    type Error = ShaderError;

    fn try_from(path: &'i Path) -> Result<Self, Self::Error> {
        let code = std::fs::read_to_string(path).map_err(ShaderError::from)?;

        Ok(Shader(Shader::preprocess(path, code)?))
    }
}

#[derive(Debug, thiserror::Error)]
pub struct ShaderError {
    pub msg: Option<String>,
    pub variant: ShaderErrorVariant,
}

impl Display for ShaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{}{}",
            self.msg
                .as_ref()
                .map(|s| format!("{s}: "))
                .unwrap_or_default(),
            self.variant,
        ))
    }
}

impl<S: ToString, E: Into<ShaderErrorVariant>> From<(S, E)> for ShaderError {
    fn from((msg, err): (S, E)) -> Self {
        Self {
            msg: Some(msg.to_string()),
            variant: err.into(),
        }
    }
}

impl<E: Into<ShaderErrorVariant>> From<E> for ShaderError {
    fn from(err: E) -> Self {
        Self {
            msg: None,
            variant: err.into(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ShaderErrorVariant {
    #[error("IOError: {0:?}")]
    /// Error when performing input/output operations like file loading
    IO(#[from] std::io::Error),
    #[error("PreProcessorDirectiveError")]
    /// Error when processing preprocessor directives
    PPD,
}

#[cfg(test)]
mod test {
    use super::*;

    use std::ffi::OsStr;

    #[test]
    fn shader_nested_include() {
        let shader = Shader::try_from(Path::new("assets/shader.wgsl"))
            .unwrap()
            .finish();
        assert!(!shader.is_empty());
    }
}
