/// Split big messages into multiple.
///
/// The resulting strings never container more than message_limit bytes
pub fn paginate_str(msg: &str, message_limit: usize) -> Vec<String> {
    let lines = msg.split('\n');
    let mut paged_mono: Vec<String> = vec![];
    for line in lines {
        // Can we still fit the new line into the last message?
        if let Some(last_paged_mono) = paged_mono.last_mut() {
            if last_paged_mono.len() + 1 + line.len() <= message_limit {
                last_paged_mono.push('\n');
                last_paged_mono.push_str(line);
                continue;
            }
        }
        // Can the line fit into its own message?
        if line.len() <= message_limit {
            paged_mono.push(line.to_string());
            continue;
        }
        // The line must be split into multiple messages.
        eprintln!("Error: ignored too long line");

        // TODO: split into multiple messages
        // This code is not utf-8 safe and might panic.
        // let mut cur_line = line.to_string();
        // while !cur_line.is_empty() {
        //     let (chunk, rest) = cur_line.split_at(std::cmp::min(cur_line.len(), MSG_LIMIT));
        //     paged_mono.push(chunk.to_string());
        //     cur_line = rest.to_string();
        // }
    }
    paged_mono
}
