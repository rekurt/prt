//! Well-known port → service name database.
//!
//! Provides a compile-time lookup table for ~170 common ports.
//! User overrides from `~/.config/prt/config.toml` take precedence.

use std::collections::HashMap;

/// Look up the service name for a port number.
///
/// Checks user overrides first, then falls back to the built-in table.
/// Returns `None` for unknown ports.
pub fn lookup(port: u16, user_overrides: &HashMap<u16, String>) -> Option<String> {
    if let Some(name) = user_overrides.get(&port) {
        return Some(name.clone());
    }
    builtin_lookup(port).map(|s| s.to_string())
}

/// Look up without allocating — returns `&'static str` for built-in ports only.
pub fn builtin_name(port: u16) -> Option<&'static str> {
    builtin_lookup(port)
}

/// Built-in port → service name lookup via match (compiles to a jump table).
fn builtin_lookup(port: u16) -> Option<&'static str> {
    match port {
        // ── Well-known system ports ──────────────────────────────
        7 => Some("echo"),
        20 => Some("ftp-data"),
        21 => Some("ftp"),
        22 => Some("ssh"),
        23 => Some("telnet"),
        25 => Some("smtp"),
        53 => Some("dns"),
        67 => Some("dhcp-s"),
        68 => Some("dhcp-c"),
        69 => Some("tftp"),
        80 => Some("http"),
        88 => Some("kerberos"),
        110 => Some("pop3"),
        111 => Some("rpcbind"),
        119 => Some("nntp"),
        123 => Some("ntp"),
        135 => Some("msrpc"),
        137 => Some("netbios"),
        138 => Some("netbios"),
        139 => Some("netbios"),
        143 => Some("imap"),
        161 => Some("snmp"),
        162 => Some("snmp-trap"),
        179 => Some("bgp"),
        194 => Some("irc"),
        389 => Some("ldap"),
        443 => Some("https"),
        445 => Some("smb"),
        465 => Some("smtps"),
        514 => Some("syslog"),
        515 => Some("lpr"),
        520 => Some("rip"),
        523 => Some("ibm-db2"),
        530 => Some("rpc"),
        543 => Some("klogin"),
        544 => Some("kshell"),
        546 => Some("dhcpv6-c"),
        547 => Some("dhcpv6-s"),
        548 => Some("afp"),
        554 => Some("rtsp"),
        587 => Some("submission"),
        631 => Some("ipp"),
        636 => Some("ldaps"),
        873 => Some("rsync"),
        902 => Some("vmware"),
        989 => Some("ftps-data"),
        990 => Some("ftps"),
        993 => Some("imaps"),
        995 => Some("pop3s"),

        // ── Common registered ports ──────────────────────────────
        1080 => Some("socks"),
        1194 => Some("openvpn"),
        1433 => Some("mssql"),
        1434 => Some("mssql-m"),
        1521 => Some("oracle"),
        1723 => Some("pptp"),
        1883 => Some("mqtt"),
        1900 => Some("ssdp"),
        2049 => Some("nfs"),
        2181 => Some("zookeeper"),
        2375 => Some("docker"),
        2376 => Some("docker-s"),
        2377 => Some("swarm"),
        2379 => Some("etcd"),
        2380 => Some("etcd-p"),
        3000 => Some("grafana"),
        3128 => Some("squid"),
        3268 => Some("gc-ldap"),
        3269 => Some("gc-ldaps"),
        3306 => Some("mysql"),
        3389 => Some("rdp"),
        3478 => Some("stun"),
        4000 => Some("remoteanything"),
        4369 => Some("epmd"),
        4443 => Some("pharos"),
        4505 => Some("salt-pub"),
        4506 => Some("salt-ret"),
        5000 => Some("upnp"),
        5060 => Some("sip"),
        5061 => Some("sip-tls"),
        5222 => Some("xmpp-c"),
        5223 => Some("xmpp-cs"),
        5228 => Some("gcm"),
        5269 => Some("xmpp-s"),
        5432 => Some("postgres"),
        5433 => Some("postgres"),
        5555 => Some("adb"),
        5601 => Some("kibana"),
        5672 => Some("amqp"),
        5683 => Some("coap"),
        5900 => Some("vnc"),
        5938 => Some("teamview"),
        5984 => Some("couchdb"),
        6379 => Some("redis"),
        6380 => Some("redis"),
        6443 => Some("k8s-api"),
        6514 => Some("syslog-t"),
        6660 => Some("irc"),
        6661 => Some("irc"),
        6662 => Some("irc"),
        6663 => Some("irc"),
        6664 => Some("irc"),
        6665 => Some("irc"),
        6666 => Some("irc"),
        6667 => Some("irc"),
        6668 => Some("irc"),
        6669 => Some("irc"),
        6881 => Some("bittorrent"),
        6969 => Some("bittorrent"),
        7000 => Some("cassandra"),
        7001 => Some("cassandra"),
        7070 => Some("realserv"),
        7199 => Some("cassandra-j"),
        7474 => Some("neo4j"),
        7687 => Some("neo4j-bolt"),
        8000 => Some("http-alt"),
        8008 => Some("http-alt"),
        8080 => Some("http-alt"),
        8081 => Some("http-alt"),
        8088 => Some("http-alt"),
        8125 => Some("statsd"),
        8200 => Some("vault"),
        8300 => Some("consul"),
        8301 => Some("consul"),
        8302 => Some("consul"),
        8333 => Some("bitcoin"),
        8443 => Some("https-alt"),
        8500 => Some("consul"),
        8761 => Some("eureka"),
        8834 => Some("nessus"),
        8888 => Some("http-alt"),
        9000 => Some("php-fpm"),
        9042 => Some("cassandra"),
        9043 => Some("websphere"),
        9090 => Some("prometheus"),
        9091 => Some("transmsn"),
        9092 => Some("kafka"),
        9093 => Some("alertmgr"),
        9100 => Some("node-exp"),
        9160 => Some("cassandra"),
        9200 => Some("elastic"),
        9300 => Some("elastic"),
        9418 => Some("git"),
        9443 => Some("https-alt"),
        9500 => Some("consul"),
        9870 => Some("hdfs"),
        9999 => Some("abyss"),
        10000 => Some("webmin"),
        10250 => Some("kubelet"),
        10255 => Some("kubelet-r"),
        11211 => Some("memcached"),
        11214 => Some("memcache"),
        11215 => Some("memcache"),
        15672 => Some("rabbit-m"),
        16379 => Some("redis-cl"),
        18080 => Some("http-alt"),
        25565 => Some("minecraft"),
        25672 => Some("rabbitmq"),
        27017 => Some("mongodb"),
        27018 => Some("mongodb"),
        27019 => Some("mongodb"),
        28017 => Some("mongodb-w"),
        29015 => Some("rethinkdb"),
        44818 => Some("etherip"),
        50000 => Some("sap"),
        50070 => Some("hdfs-web"),
        61613 => Some("stomp"),
        61616 => Some("activemq"),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_well_known_ports() {
        assert_eq!(builtin_lookup(22), Some("ssh"));
        assert_eq!(builtin_lookup(80), Some("http"));
        assert_eq!(builtin_lookup(443), Some("https"));
        assert_eq!(builtin_lookup(5432), Some("postgres"));
        assert_eq!(builtin_lookup(6379), Some("redis"));
        assert_eq!(builtin_lookup(9092), Some("kafka"));
    }

    #[test]
    fn builtin_unknown_port() {
        assert_eq!(builtin_lookup(12345), None);
        assert_eq!(builtin_lookup(0), None);
        assert_eq!(builtin_lookup(65535), None);
    }

    #[test]
    fn user_override_takes_precedence() {
        let mut overrides = HashMap::new();
        overrides.insert(80, "my-proxy".to_string());
        assert_eq!(lookup(80, &overrides).as_deref(), Some("my-proxy"));
    }

    #[test]
    fn fallback_to_builtin_when_no_override() {
        let overrides = HashMap::new();
        assert_eq!(lookup(80, &overrides).as_deref(), Some("http"));
        assert_eq!(lookup(12345, &overrides), None);
    }

    #[test]
    fn override_for_unknown_port() {
        let mut overrides = HashMap::new();
        overrides.insert(9999, "custom-svc".to_string());
        assert_eq!(lookup(9999, &overrides).as_deref(), Some("custom-svc"));
    }
}
