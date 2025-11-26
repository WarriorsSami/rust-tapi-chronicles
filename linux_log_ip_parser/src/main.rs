use std::collections::BTreeMap;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio_stream::StreamExt;
use tokio_util::io::StreamReader;

const LOG_FILE_URL: &str =
    "https://raw.githubusercontent.com/logpai/loghub/refs/heads/master/Linux/Linux_2k.log";
const OUTPUT_FILE_PATH: &str = "Linux2k_IP_stat.txt";
const IPV4_REGEX: &str = r"(25[0-5]|2[0-4]\d|[01]?\d?\d)[\.-](25[0-5]|2[0-4]\d|[01]?\d?\d)[\.-](25[0-5]|2[0-4]\d|[01]?\d?\d)[\.-](25[0-5]|2[0-4]\d|[01]?\d?\d)";

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct IPv4Address {
    first_octet: u8,
    second_octet: u8,
    third_octet: u8,
    fourth_octet: u8,
}

impl IPv4Address {
    pub fn try_parse(ip_str: &str) -> Option<Self> {
        // use the regex to extract the octets
        let regex = regex::Regex::new(IPV4_REGEX).unwrap();
        let captures = regex.captures(ip_str)?;

        let first_octet = captures[1].parse::<u8>().ok()?;
        let second_octet = captures[2].parse::<u8>().ok()?;
        let third_octet = captures[3].parse::<u8>().ok()?;
        let fourth_octet = captures[4].parse::<u8>().ok()?;

        Some(Self {
            first_octet,
            second_octet,
            third_octet,
            fourth_octet,
        })
    }
}

impl std::fmt::Display for IPv4Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}",
            self.first_octet, self.second_octet, self.third_octet, self.fourth_octet
        )
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let response = reqwest::get(LOG_FILE_URL).await?.error_for_status()?;

    let byte_stream = response
        .bytes_stream()
        .map(|result| result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)));
    let stream_reader = StreamReader::new(byte_stream);
    let mut reader = BufReader::new(stream_reader);

    let pwd = std::env::current_dir()?;
    let output_file_path = pwd.join("output").join(OUTPUT_FILE_PATH);
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(output_file_path)
        .await?;
    let ip_regex = regex::Regex::new(IPV4_REGEX).unwrap();
    let mut ip_table = BTreeMap::<IPv4Address, u32>::new();

    let mut line = String::new();
    while reader.read_line(&mut line).await? > 0 {
        // extract all the IP addresses from the log line using regex and count the occurrences of each IP address
        let ip_addresses = ip_regex
            .find_iter(&line)
            .map(|m| m.as_str())
            .map(IPv4Address::try_parse)
            .flatten()
            .collect::<Vec<_>>();
        ip_addresses.iter().for_each(|ip| {
            let count = ip_table.entry(ip.clone()).or_insert(0);
            *count += 1;
        });

        line.clear();
    }

    // dump the IP address and count pairs to the output file
    for (ip, count) in ip_table {
        // use a fixed-width field width of 10 to align the output
        file.write_all(format!("{:<15} {}\n", ip, count).as_bytes())
            .await?;
    }

    Ok(())
}
