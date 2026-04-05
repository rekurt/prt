//! Network namespace awareness (Linux only).
//!
//! Groups processes by their network namespace inode. Named namespaces
//! from `/run/netns/` get human-readable labels; unnamed ones show the
//! raw inode number.
//!
//! On non-Linux platforms, all functions return empty/default results.

use std::collections::HashMap;

/// A network namespace with its inode and optional human-readable name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetNamespace {
    /// The namespace inode number (unique identifier).
    pub inode: u64,
    /// Human-readable name from /run/netns/, if available.
    pub name: Option<String>,
}

impl NetNamespace {
    /// Display label: name if available, otherwise "ns:<inode>".
    pub fn label(&self) -> String {
        match &self.name {
            Some(n) => n.clone(),
            None => format!("ns:{}", self.inode),
        }
    }
}

/// Resolve the network namespace for a given PID.
/// Returns None on non-Linux or if the namespace can't be read.
pub fn resolve_namespace(pid: u32) -> Option<NetNamespace> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let inode = read_ns_inode(pid)?;
    Some(NetNamespace { inode, name: None })
}

/// Batch-resolve namespaces for multiple PIDs.
/// Returns a map from PID to namespace.
pub fn resolve_namespaces(pids: &[u32]) -> HashMap<u32, NetNamespace> {
    let mut result = HashMap::new();
    if !cfg!(target_os = "linux") {
        return result;
    }

    // First, build a map of named namespaces from /run/netns/
    let named = load_named_namespaces();

    for &pid in pids {
        if let Some(inode) = read_ns_inode(pid) {
            let name = named.get(&inode).cloned();
            result.insert(pid, NetNamespace { inode, name });
        }
    }

    result
}

/// Group PIDs by their namespace inode.
/// Returns Vec<(namespace, Vec<pid>)> sorted by namespace label.
pub fn group_by_namespace(pid_ns: &HashMap<u32, NetNamespace>) -> Vec<(NetNamespace, Vec<u32>)> {
    let mut by_inode: HashMap<u64, (NetNamespace, Vec<u32>)> = HashMap::new();

    for (&pid, ns) in pid_ns {
        by_inode
            .entry(ns.inode)
            .or_insert_with(|| (ns.clone(), Vec::new()))
            .1
            .push(pid);
    }

    let mut groups: Vec<_> = by_inode.into_values().collect();
    groups.sort_by(|a, b| a.0.label().cmp(&b.0.label()));
    for (_, pids) in &mut groups {
        pids.sort();
    }
    groups
}

/// Read the network namespace inode for a PID from /proc/{pid}/ns/net.
#[allow(dead_code)]
fn read_ns_inode(pid: u32) -> Option<u64> {
    let link = std::fs::read_link(format!("/proc/{pid}/ns/net")).ok()?;
    let s = link.to_string_lossy();
    // Format: "net:[<inode>]"
    parse_ns_inode(&s)
}

/// Parse inode from a readlink result like "net:[4026531992]".
fn parse_ns_inode(s: &str) -> Option<u64> {
    let start = s.find('[')?;
    let end = s.find(']')?;
    s[start + 1..end].parse().ok()
}

/// Load named namespaces from /run/netns/.
/// Returns a map from inode → name.
/// Files in /run/netns/ are bind-mounts (not symlinks), so we use
/// metadata().ino() to get the namespace inode.
#[allow(dead_code)]
fn load_named_namespaces() -> HashMap<u64, String> {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::MetadataExt;
        let mut result = HashMap::new();
        if let Ok(dir) = std::fs::read_dir("/run/netns") {
            for entry in dir.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if let Ok(meta) = std::fs::metadata(format!("/run/netns/{name}")) {
                    result.insert(meta.ino(), name);
                }
            }
        }
        result
    }

    #[cfg(not(target_os = "linux"))]
    HashMap::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ns_inode_valid() {
        assert_eq!(parse_ns_inode("net:[4026531992]"), Some(4026531992));
    }

    #[test]
    fn parse_ns_inode_invalid() {
        assert_eq!(parse_ns_inode("garbage"), None);
        assert_eq!(parse_ns_inode("net:[]"), None);
        assert_eq!(parse_ns_inode("net:[abc]"), None);
    }

    #[test]
    fn namespace_label_with_name() {
        let ns = NetNamespace {
            inode: 123,
            name: Some("myns".into()),
        };
        assert_eq!(ns.label(), "myns");
    }

    #[test]
    fn namespace_label_without_name() {
        let ns = NetNamespace {
            inode: 4026531992,
            name: None,
        };
        assert_eq!(ns.label(), "ns:4026531992");
    }

    #[test]
    fn group_by_namespace_groups_correctly() {
        let mut pid_ns = HashMap::new();
        let ns1 = NetNamespace {
            inode: 100,
            name: Some("default".into()),
        };
        let ns2 = NetNamespace {
            inode: 200,
            name: Some("container".into()),
        };
        pid_ns.insert(1, ns1.clone());
        pid_ns.insert(2, ns1.clone());
        pid_ns.insert(3, ns2.clone());

        let groups = group_by_namespace(&pid_ns);
        assert_eq!(groups.len(), 2);

        // Sorted by label: "container" < "default"
        assert_eq!(groups[0].0.name, Some("container".into()));
        assert_eq!(groups[0].1, vec![3]);
        assert_eq!(groups[1].0.name, Some("default".into()));
        assert_eq!(groups[1].1, vec![1, 2]);
    }

    #[test]
    fn group_by_namespace_empty() {
        let groups = group_by_namespace(&HashMap::new());
        assert!(groups.is_empty());
    }

    #[test]
    fn resolve_namespaces_non_linux_returns_empty() {
        if !cfg!(target_os = "linux") {
            let result = resolve_namespaces(&[1, 2, 3]);
            assert!(result.is_empty());
        }
    }
}
