use std::{fmt, process::Command};
use serde::Deserialize;


/// Runs `zypper --xmlout lp`, optionally including all historical patches with
/// `-a`, and deserializes its XML response.
pub fn list_zypper_patches(include_all: bool) -> Result<PatchList, ZypperError> {
    let mut command = Command::new("zypper");
    command.args(["--xmlout", "lp"]);
    if include_all {
        command.arg("-a");
    }

    let output = command.output().map_err(ZypperError::Start)?;

    if !output.status.success() {
        return Err(ZypperError::Failed {
            status: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    quick_xml::de::from_reader(output.stdout.as_slice()).map_err(ZypperError::InvalidXml)
}


/// XML document returned by zypper's `lp` command. `messages` includes zypper
/// progress and error messages; patches are in `update_status.update_list`.
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename = "stream")]
pub struct PatchList {
    #[serde(rename = "message", default)]
    pub messages: Vec<PatchMessage>,
    #[serde(rename = "update-status")]
    pub update_status: Option<PatchUpdateStatus>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct PatchMessage {
    #[serde(rename = "@type")]
    pub kind: String,
    #[serde(rename = "$text", default)]
    pub text: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct PatchUpdateStatus {
    #[serde(rename = "@version")]
    pub version: String,
    #[serde(rename = "update-list")]
    pub update_list: PatchUpdateList,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct PatchUpdateList {
    #[serde(rename = "update", default)]
    pub updates: Vec<Patch>,
}

/// A patch entry reported by zypper.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct Patch {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@kind")]
    pub kind: String,
    #[serde(rename = "@edition")]
    pub edition: String,
    #[serde(rename = "@arch")]
    pub arch: String,
    #[serde(rename = "@status", default)]
    pub status: Option<String>,
    #[serde(rename = "@category", default)]
    pub category: Option<String>,
    #[serde(rename = "@severity", default)]
    pub severity: Option<String>,
    pub summary: String,
    pub description: String,
    pub license: String,
    pub source: RepoSource,
    pub issue_date: PatchIssueDate,
    pub issue_list: PatchIssueList,
}

/// Repository metadata emitted as a `<source>` element inside every update.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct RepoSource {
    #[serde(rename = "@url")]
    pub url: String,
    #[serde(rename = "@alias")]
    pub alias: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct PatchIssueDate {
    #[serde(rename = "@text")]
    pub date: String,
    #[serde(rename = "@time_t")]
    pub time_t: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct PatchIssueList {
    #[serde(rename = "issue", default)]
    pub issue: Vec<PatchIssue>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct PatchIssue {
    #[serde(rename = "@id")]
    pub issue_id: String,
    #[serde(rename = "@type")]
    pub issue_type: String,
    pub title: String,
    pub href: String,
}
#[derive(Debug)]
pub enum ZypperError {
    Start(std::io::Error),
    Failed { status: Option<i32>, stderr: String },
    InvalidXml(quick_xml::DeError),
}

impl fmt::Display for ZypperError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Start(error) => write!(f, "could not start zypper: {error}"),
            Self::Failed { status, stderr } => {
                write!(f, "zypper failed with status {status:?}: {stderr}")
            }
            Self::InvalidXml(error) => write!(f, "zypper returned invalid XML: {error}"),
        }
    }
}

impl std::error::Error for ZypperError {}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_zypper_patch_list() {
        let xml = r#"<?xml version="1.0"?>
            <stream>
              <message type="info">Loading repository data...</message>
              <update-status version="0.6">
               <update-list>
                <update name="openSUSE-SLE-15.6-2026-1"
                        kind="patch"
                        edition="1"
                        arch="noarch"
                        status="needed"
                        category="security"
                        severity="important">
                  <summary>An important update</summary>
                  <description>Patch details</description>
                  <license>MIT</license>
                  <source alias="Main Repository" url="https://example.invalid/repo" />
                  <issue-date text="2026-06-21T00:44:54Z" time_t="1782002694"/>
                  <issue-list>
                  <issue id="1266598" type="bugzilla">
            <title>VUL-0: CVE-2026-39821: helm: golang.org/x/net/idna: failure to reject ASCII-only Punycode-encoded labels allows for validation bypass and privilege escalation</title>
            <href>https://bugzilla.suse.com/show_bug.cgi?id=1266598</href>
          </issue>
          <issue id="CVE-2026-39821" type="cve">
            <title>https://cve.mitre.org/cgi-bin/cvename.cgi?name=CVE-2026-39821</title>
            <href>https://cve.mitre.org/cgi-bin/cvename.cgi?name=CVE-2026-39821</href>
          </issue>
        </issue-list>
                </update>
               </update-list>
               <blocked-update-list />
              </update-status>
            </stream>"#;

        let patches: PatchList = quick_xml::de::from_str(xml).unwrap();

        assert_eq!(patches.messages[0].kind, "info");
        assert_eq!(patches.messages[0].text, "Loading repository data...");
        let patch = &patches.update_status.unwrap().update_list.updates[0];
        assert_eq!(patch.name, "openSUSE-SLE-15.6-2026-1");
        assert_eq!(patch.status.as_deref(), Some("needed"));
        assert_eq!(patch.category.as_deref(), Some("security"));
        assert_eq!(patch.summary, "An important update");
        assert_eq!(patch.source.alias, "Main Repository");
        assert!(!patch.issue_list.issue.is_empty());
    }
}
