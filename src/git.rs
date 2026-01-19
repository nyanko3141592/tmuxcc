//! Git repository information utilities

use std::fs;
use std::path::Path;

/// Get git branch name for a given path
///
/// This function looks for the .git directory and reads the HEAD file
/// to determine the current branch name.
pub fn get_git_branch(path: &str) -> Option<String> {
    if path.is_empty() {
        return None;
    }

    let path = Path::new(path);

    // Walk up the directory tree to find .git
    let mut current = Some(path);
    while let Some(dir) = current {
        let git_dir = dir.join(".git");

        if git_dir.is_dir() {
            // Found a .git directory, read HEAD
            let head_path = git_dir.join("HEAD");
            if let Ok(contents) = fs::read_to_string(&head_path) {
                return parse_git_head(&contents);
            }
        } else if git_dir.is_file() {
            // .git might be a file (worktree), read it to find the actual git dir
            if let Ok(contents) = fs::read_to_string(&git_dir) {
                if let Some(git_path) = contents.strip_prefix("gitdir: ") {
                    let git_path = git_path.trim();
                    let head_path = Path::new(git_path).join("HEAD");
                    if let Ok(head_contents) = fs::read_to_string(&head_path) {
                        return parse_git_head(&head_contents);
                    }
                }
            }
        }

        current = dir.parent();
    }

    None
}

/// Parse the contents of a .git/HEAD file to extract the branch name
fn parse_git_head(contents: &str) -> Option<String> {
    let contents = contents.trim();

    // Check if HEAD is a symbolic reference (ref: refs/heads/branch-name)
    if let Some(ref_path) = contents.strip_prefix("ref: ") {
        // Extract branch name from refs/heads/branch-name
        if let Some(branch) = ref_path.strip_prefix("refs/heads/") {
            return Some(branch.to_string());
        }
        // Could be refs/remotes/origin/branch-name for detached tracking
        if let Some(remote_branch) = ref_path.strip_prefix("refs/remotes/") {
            return Some(remote_branch.to_string());
        }
        return Some(ref_path.to_string());
    }

    // HEAD is a detached commit hash - return short hash
    if contents.len() >= 7 && contents.chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(format!("{}...", &contents[..7]));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_git_head_branch() {
        assert_eq!(
            parse_git_head("ref: refs/heads/main"),
            Some("main".to_string())
        );
        assert_eq!(
            parse_git_head("ref: refs/heads/feature/new-feature"),
            Some("feature/new-feature".to_string())
        );
    }

    #[test]
    fn test_parse_git_head_detached() {
        assert_eq!(
            parse_git_head("abc1234567890abcdef"),
            Some("abc1234...".to_string())
        );
    }

    #[test]
    fn test_parse_git_head_with_newline() {
        assert_eq!(
            parse_git_head("ref: refs/heads/main\n"),
            Some("main".to_string())
        );
    }

    #[test]
    fn test_get_git_branch_empty_path() {
        assert_eq!(get_git_branch(""), None);
    }
}
