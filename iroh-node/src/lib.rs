use anyhow::Result;
use futures_lite::StreamExt;
use iroh::{Endpoint, NodeAddr, NodeId, protocol::Router};
use iroh_gossip::{
    net::{Event, Gossip, GossipEvent, GossipReceiver},
    proto::TopicId,
};

#[tokio::main]
async fn main() -> Result<()> {
    let topic = TopicId::from_bytes(rand::random());
    println!("> opening chat room for topic {topic}");
    let endpoint = Endpoint::builder().discovery_n0().bind().await?;

    println!("> our node id: {}", endpoint.node_id());
    let gossip = Gossip::builder().spawn(endpoint.clone()).await?;

    let router = Router::builder(endpoint.clone())
        .accept(iroh_gossip::ALPN, gossip.clone())
        .spawn();

    // in our main file, after we create a topic `id`:
    // print a ticket that includes our own node id and endpoint addresses
    let Ticket { topic, nodes } = Ticket::from_str("")?;

    let node_ids = nodes.iter().map(|p| p.node_id).collect();
    if nodes.is_empty() {
        println!("> waiting for nodes to join us...");
    } else {
        println!("> trying to connect to {} nodes...", nodes.len());
        // add the peer addrs from the ticket to our endpoint's addressbook so that they can be dialed
        for node in nodes.into_iter() {
            endpoint.add_node_addr(node)?;
        }
    };
    let (sender, receiver) = gossip.subscribe_and_join(topic, node_ids).await?.split();
    println!("> connected!");

    // subscribe and print loop
    let my_node_id = endpoint.node_id();
    tokio::spawn(subscribe_loop(receiver, my_node_id, "lee".clone()));

    // spawn an input thread that reads stdin
    // create a multi-provider, single-consumer channel
    let (line_tx, mut line_rx) = tokio::sync::mpsc::channel(1);
    // and pass the `sender` portion to the `input_loop`
    std::thread::spawn(move || input_loop(line_tx));
}

// Handle incoming events
async fn subscribe_loop(
    mut receiver: GossipReceiver,
    my_node_id: NodeId,
    my_name: Option<String>,
) -> Result<()> {
    // keep track of the mapping between `NodeId`s and names
    let mut names = HashMap::new();
    // 反向映射：从名字到NodeId
    let mut name_to_node = HashMap::new();

    // 如果我们有自己的昵称，先添加到映射中
    if let Some(name) = &my_name {
        names.insert(my_node_id, name.clone());
        name_to_node.insert(name.clone(), my_node_id);
    }

    // iterate over all events
    while let Some(event) = receiver.try_next().await? {
        // if the Event is a `GossipEvent::Received`, let's deserialize the message:
        if let Event::Gossip(GossipEvent::Received(msg)) = event {
            // deserialize the message and match on the
            // message type:
        }
    }
    Ok(())
}

fn input_loop(line_tx: tokio::sync::mpsc::Sender<String>) -> Result<()> {
    let mut buffer = String::new();
    let stdin = std::io::stdin(); // We get `Stdin` here.
    loop {
        stdin.read_line(&mut buffer)?;
        line_tx.blocking_send(buffer.clone())?;
        buffer.clear();
    }
}

/// 解析消息中的 @mentions
fn parse_mentions(text: &str) -> Vec<String> {
    let mut mentions = Vec::new();
    let words: Vec<&str> = text.split_whitespace().collect();

    for word in words {
        if word.starts_with('@') && word.len() > 1 {
            let username = &word[1..]; // 去掉 @ 符号
            // 移除可能的标点符号
            let clean_username =
                username.trim_end_matches(&[',', '.', '!', '?', ':', ';', ')', ']', '}'][..]);
            if !clean_username.is_empty() {
                mentions.push(clean_username.to_string());
            }
        }
    }

    // 去重
    mentions.sort();
    mentions.dedup();
    mentions
}

/// 解析私人消息格式：/username message
/// 返回：(目标用户名, 消息内容)，如果不是私人消息格式则返回 None
fn parse_private_message(text: &str) -> Option<(String, String)> {
    let text = text.trim();
    if text.starts_with('/') && text.len() > 1 {
        // 找到第一个空格的位置
        if let Some(space_pos) = text.find(' ') {
            let username = &text[1..space_pos]; // 去掉 / 符号
            let message = text[space_pos + 1..].trim();

            if !username.is_empty() && !message.is_empty() {
                return Some((username.to_string(), message.to_string()));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mentions() {
        // 测试基本提及
        assert_eq!(parse_mentions("Hello @alice"), vec!["alice"]);

        // 测试多个提及
        assert_eq!(
            parse_mentions("@bob @charlie hello"),
            vec!["bob", "charlie"]
        );

        // 测试带标点符号的提及
        assert_eq!(parse_mentions("Thanks @alice!"), vec!["alice"]);
        assert_eq!(parse_mentions("Hey @bob, how are you?"), vec!["bob"]);

        // 测试重复提及
        assert_eq!(parse_mentions("@alice @alice"), vec!["alice"]);

        // 测试空字符串和无提及
        assert_eq!(parse_mentions(""), Vec::<String>::new());
        assert_eq!(parse_mentions("Hello world"), Vec::<String>::new());

        // 测试单独的@符号
        assert_eq!(parse_mentions("@"), Vec::<String>::new());
    }

    #[test]
    fn test_parse_private_message() {
        // 测试基本私人消息
        assert_eq!(
            parse_private_message("/alice Hello there!"),
            Some(("alice".to_string(), "Hello there!".to_string()))
        );

        // 测试带空格的消息
        assert_eq!(
            parse_private_message("/bob How are you doing today?"),
            Some(("bob".to_string(), "How are you doing today?".to_string()))
        );

        // 测试前后有空格的情况
        assert_eq!(
            parse_private_message("  /charlie  Hi!  "),
            Some(("charlie".to_string(), "Hi!".to_string()))
        );

        // 测试非私人消息格式
        assert_eq!(parse_private_message("Hello world"), None);
        assert_eq!(parse_private_message("@alice hello"), None);
        assert_eq!(parse_private_message(""), None);

        // 测试无效格式
        assert_eq!(parse_private_message("/"), None);
        assert_eq!(parse_private_message("/alice"), None); // 没有消息内容
        assert_eq!(parse_private_message("/ hello"), None); // 没有用户名
    }
}

// add the `Ticket` code to the bottom of the main file
#[derive(Debug, Serialize, Deserialize)]
struct Ticket {
    topic: TopicId,
    nodes: Vec<NodeAddr>,
}

impl Ticket {
    /// Deserialize from a slice of bytes to a Ticket.
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(Into::into)
    }

    /// Serialize from a `Ticket` to a `Vec` of bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("serde_json::to_vec is infallible")
    }
}

// The `Display` trait allows us to use the `to_string`
// method on `Ticket`.
impl fmt::Display for Ticket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut text = data_encoding::BASE32_NOPAD.encode(&self.to_bytes()[..]);
        text.make_ascii_lowercase();
        write!(f, "{}", text)
    }
}

// The `FromStr` trait allows us to turn a `str` into
// a `Ticket`
impl FromStr for Ticket {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = data_encoding::BASE32_NOPAD.decode(s.to_ascii_uppercase().as_bytes())?;
        Self::from_bytes(&bytes)
    }
}
