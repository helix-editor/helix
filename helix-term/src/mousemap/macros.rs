#[macro_export]
macro_rules! mousemap {
    (@trie $cmd:ident) => {
        $crate::mousemap::MouseTrie::MappableCommand($crate::commands::mouse::StaticMouseCommand::$cmd)
    };

    (@trie [$($cmd:ident),* $(,)?]) => {
        $crate::mousemap::MouseTrie::Sequence(vec![$($crate::commands::mouse::StaticMouseCommand::$cmd),*])
    };

    (
        {  $($key:literal => $value:tt,)+ }
    ) => {
        // modified from the hashmap! macro
        {
            let _cap = hashmap!(@count $($key),*);
            let mut _map = ::std::collections::HashMap::with_capacity(_cap);
            $(
                let _key = $key.parse::<::helix_view::input::MouseEvent>().unwrap();
                let _duplicate = _map.insert(
                    _key,
                    mousemap!(@trie $value)
                );
                assert!(_duplicate.is_none(), "Duplicate key found: {:?}", _duplicate.unwrap());
            )+
            _map
        }
    };
}
