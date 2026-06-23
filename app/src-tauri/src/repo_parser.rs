use regex::Regex;

pub struct ParsedRepoRef {
    pub owner: String,
    pub repo: String,
    pub normalized_url: String,
}

pub fn parse_repo_ref(input: &str) -> Result<ParsedRepoRef, String> {
    let input = input.trim().trim_end_matches('/');
    if input.is_empty() {
        return Err("Enter a GitHub repository URL, such as https://github.com/owner/repo".to_string());
    }

    if input.contains("/pull/") {
        return Err("Pull request URLs are not supported here. Enter a repository URL like https://github.com/owner/repo".to_string());
    }

    let full_re = Regex::new(r"^https?://github\.com/([^/]+)/([^/#?]+)(?:[/?#].*)?$").unwrap();
    let host_re = Regex::new(r"^github\.com/([^/]+)/([^/#?]+)(?:[/?#].*)?$").unwrap();
    let short_re = Regex::new(r"^([^/\s]+)/([^/#?\s]+)$").unwrap();

    let caps = full_re
        .captures(input)
        .or_else(|| host_re.captures(input))
        .or_else(|| short_re.captures(input));

    let Some(caps) = caps else {
        return Err(format!(
            "Could not parse repository reference: '{}'. Expected https://github.com/owner/repo or owner/repo",
            input
        ));
    };

    let owner = caps[1].trim().to_string();
    let repo = caps[2].trim().trim_end_matches(".git").to_string();

    if owner.is_empty() || repo.is_empty() || owner == "." || repo == "." {
        return Err("Repository reference must include both owner and repository name.".to_string());
    }

    Ok(ParsedRepoRef {
        normalized_url: format!("https://github.com/{}/{}", owner, repo),
        owner,
        repo,
    })
}
