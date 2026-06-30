use std::path::Path;

pub fn shell_quote(input: &str) -> String {
    if input.is_empty() {
        return "''".to_owned();
    }
    format!("'{}'", input.replace('\'', "'\\''"))
}

pub fn cd_command(path: &Path) -> String {
    format!("cd {}", shell_quote(&path.display().to_string()))
}

pub fn functions_block() -> &'static str {
    r#"@()        { eval "$(at menu --shell "$@")"; }
@recent()  { eval "$(at recent --shell "$@")"; }
@flow()    { eval "$(at flow --shell "$@")"; }
@search()  { eval "$(at search --shell "$@")"; }
@setting() { at setting "$@"; }"#
}

pub fn cd_hook_block() -> &'static str {
    r#"_atflow_record_cd() {
  command at recent-record "$PWD" >/dev/null 2>&1 || true
}

if [ -n "${ZSH_VERSION:-}" ]; then
  autoload -Uz add-zsh-hook
  add-zsh-hook chpwd _atflow_record_cd
elif [ -n "${BASH_VERSION:-}" ]; then
  _atflow_original_cd() {
    builtin cd "$@" && _atflow_record_cd
  }
  alias cd='_atflow_original_cd'
fi"#
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn quotes_paths_for_shell_eval() {
        assert_eq!(shell_quote("/home/a b/project"), "'/home/a b/project'");
        assert_eq!(
            shell_quote("/home/it's/project"),
            "'/home/it'\\''s/project'"
        );
    }

    #[test]
    fn cd_command_wraps_quoted_path() {
        assert_eq!(
            cd_command(&PathBuf::from("/home/congke/work/at flow")),
            "cd '/home/congke/work/at flow'"
        );
    }

    #[test]
    fn functions_include_user_facing_entries() {
        let block = functions_block();
        assert!(block.contains("@()"));
        assert!(block.contains("@recent()"));
        assert!(block.contains("@flow()"));
        assert!(block.contains("@search()"));
    }
}
