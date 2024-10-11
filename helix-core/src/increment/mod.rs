mod date_time;
mod integer;
mod string;

pub fn integer(selected_text: &str, amount: i64) -> Option<String> {
    integer::increment(selected_text, amount)
}

pub fn date_time(selected_text: &str, amount: i64) -> Option<String> {
    date_time::increment(selected_text, amount)
}

pub fn string(selected_text: &str, amount: i64) -> Option<String> {
    string::increment(selected_text, amount)
}
