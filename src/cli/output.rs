use crate::api::ConversationsResponse;
use crate::types::*;
use serde::Serialize;

#[derive(Serialize)]
pub struct ConversationDetail<'a> {
    #[serde(flatten)]
    pub conversation: &'a Conversation,
    pub messages: &'a [Message],
}

pub fn format_conversations(response: &ConversationsResponse) {
    print_json(response);
}

pub fn format_conversation_detail(conv: &Conversation, messages: &[Message]) {
    print_json(&ConversationDetail {
        conversation: conv,
        messages,
    });
}

pub fn format_folders(folders: &[Folder]) {
    print_json(folders);
}

pub fn format_tags(tags: &[Tag]) {
    print_json(tags);
}

pub fn format_canned_replies(replies: &[CannedReply]) {
    print_json(replies);
}

pub fn format_canned_reply(reply: &CannedReply) {
    print_json(reply);
}

pub fn format_agent(agent: &CurrentAgent) {
    print_json(agent);
}

fn print_json(value: &(impl Serialize + ?Sized)) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).expect("serialization should not fail")
    );
}
