use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio_stream::StreamExt;
use tokio_util::io::StreamReader;

const LOG_FILE_URL: &str =
    "https://raw.githubusercontent.com/logpai/loghub/refs/heads/master/Apache/Apache_2k.log";
const OUTPUT_DIR_PATH: &str = "output";
const KEYWORD_REGEX: &str = r"^\[.*?\]\s*\[([^\]]+)\]";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let response = reqwest::get(LOG_FILE_URL).await?.error_for_status()?;

    let byte_stream = response
        .bytes_stream()
        .map(|result| result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)));
    let stream_reader = StreamReader::new(byte_stream);
    let mut reader = BufReader::new(stream_reader);

    let pwd = std::env::current_dir()?;
    let output_dir_path = pwd.join(OUTPUT_DIR_PATH);

    let mut line = String::new();
    while reader.read_line(&mut line).await? > 0 {
        // extract the keyword from the second column of the log line using regex
        // e.g.: [text1 text2 text3] [keyword] [text]
        match regex::Regex::new(KEYWORD_REGEX)
            .unwrap()
            .captures(&line)
            .map(|cap| cap.get(1).unwrap().as_str())
            .into_iter()
            .next()
            .map(String::from)
        {
            Some(keyword) => {
                let output_file_path = output_dir_path.join(format!("Apache_2k-[{}].txt", keyword));
                // create the file if not existing, otherwise append the line to the file
                let mut file = tokio::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(output_file_path)
                    .await?;

                file.write_all(line.as_bytes()).await?;
            }
            None => {}
        }

        line.clear();
    }

    Ok(())
}
