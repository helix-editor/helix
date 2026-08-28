; Threading headers provide useful definitions and references.
(message_id_field
  value: (msg_id) @name) @definition.message

(in_reply_to_field
  (msg_id) @name) @reference.reply

(references_field
  (msg_id) @name) @reference.thread
