//! These are macros to make getting very nested fields in the `Editor` struct easier
//! These are macros instead of functions because functions will have to take `&mut self`
//! However, rust doesn't know that you only want a partial borrow instead of borrowing the
//! entire struct which `&mut self` says.  This makes it impossible to do other mutable
//! stuff to the struct because it is already borrowed. Because macros are expanded,
//! this circumvents the problem because it is just like indexing fields by hand and then
//! putting a `&mut` in front of it. This way rust can see that we are only borrowing a
//! part of the struct and not the entire thing.

/// Get the current view and document mutably as a tuple.
/// Returns `(&mut View, &mut Document)`
#[macro_export]
macro_rules! current {
    ($editor:expr, $client_id:expr) => {{
        let client = $crate::client_mut!($editor, $client_id);
        let view = $editor.views.get_mut(client.tree.focus);
        let id = view.doc;
        let doc = $crate::doc_with_id_mut!($editor, &id);
        (client, view, doc)
    }};
}

#[macro_export]
macro_rules! current_ref {
    ($editor:expr, $client_id:expr) => {{
        let client = $crate::client!($editor, $client_id);
        let view = $editor.views.get(client.tree.focus);
        let id = view.doc;
        let doc = $crate::doc_with_id!($editor, &id);
        (client, view, doc)
    }};
}

/// Get the current document mutably.
/// Returns `&mut Document`
#[macro_export]
macro_rules! doc_mut {
    ($editor:expr, $client_id:expr) => {{
        $crate::current!($editor, $client_id).2
    }};
}

#[macro_export]
macro_rules! doc_with_id_mut {
    ($editor:expr, $id:expr) => {{
        $editor.documents.get_mut($id).unwrap()
    }};
}

#[macro_export]
macro_rules! client {
    ($editor:expr, $id:expr) => {{
        &$editor.clients[$id]
    }};
}

#[macro_export]
macro_rules! client_mut {
    ($editor:expr, $id:expr) => {{
        $editor.clients.get_mut($id).unwrap()
    }};
}

/// Get the current view mutably.
/// Returns `&mut View`
#[macro_export]
macro_rules! view_mut {
    ($editor:expr, $view_id:expr) => {{
        $editor.views.get_mut($view_id)
    }};
}

/// Get the current view immutably
/// Returns `&View`
#[macro_export]
macro_rules! view {
    ($editor:expr, $view_id:expr) => {{
        $editor.views.get($view_id)
    }};
}

/// Get the current view mutably.
/// Returns `&mut View`
#[macro_export]
macro_rules! client_view_mut {
    ($editor:expr, $client_id:expr) => {{
        let client = $crate::client_mut!($editor, $client_id);
        $editor.views.get_mut(client.tree.focus)
    }};
}

/// Get the current view immutably
/// Returns `&View`
#[macro_export]
macro_rules! client_view {
    ($editor:expr, $client_id:expr) => {{
        let client = &mut $crate::client!($editor, $client_id);
        $editor.views.get(client.tree.focus)
    }};
}

#[macro_export]
macro_rules! doc {
    ($editor:expr, $client_id:expr) => {{
        $crate::current_ref!($editor, $client_id).2
    }};
}

#[macro_export]
macro_rules! doc_with_id {
    ($editor:expr, $id:expr) => {{
        &$editor.documents[$id]
    }};
}
