/// This macro allows specifiying a trait of related config
/// options with a struct like syntax. From that information
/// two things are generated:
///
/// * A `init_config` function that registers the config options with the
///   `OptionRegistry` registry.
/// * A **trait** definition with an accessor for every config option that is
///   implemented for `OptionManager`.
///
/// The accessors on the trait allow convenient statically typed access to
/// config fields. The accessors return `Guard<T>` (which allows derferecning to
/// &T). Any type that implements copy can be returned as a copy instead by
/// specifying `#[read = copy]`. Collections like `List<T>` and `String` are not
/// copy However, they usually implement deref (to &[T] and &str respectively).
/// Working with the dereferneced &str/&[T] is more convenient then &String and &List<T>. The
/// accessor will return these if `#[read = deref]` is specified.
///
/// The doc comments will be retained for the accessors and also stored in the
/// option registrry for dispaly in the UI and documentation.
///
/// The name of a config option can be changed with #[name = "<name>"],
/// otherwise the name of the field is used directly. The OptionRegistry
/// automatically converts all names to kebab-case so a name attribute is only
/// required if the name is supposed to be significantly altered.
///
/// In some cases more complex validation may be necssary. In that case the
/// valiidtator can be provided with an exprission that implements the `Validator`
/// trait: `#[validator = create_validator()]`.
#[macro_export]
macro_rules! options {
    (
        $(use $use: ident::*;)*
        $($(#[$($meta: tt)*])* struct $ident: ident {
            $(
                $(#[doc = $option_desc: literal])*
                $(#[name = $option_name: literal])?
                $(#[validator = $option_validator: expr])?
                $(#[read = $($extra: tt)*])?
                $option: ident: $ty: ty = $default: expr
            ),+$(,)?
        })+
    ) => {
        $(pub use $use::*;)*
        $($(#[$($meta)*])* pub trait $ident {
            $(
                $(#[doc = $option_desc])*
                fn $option(&self) -> $crate::options!(@ret_ty $($($extra)*)? $ty);
            )+
        })+
        pub fn init_config(registry: &mut $crate::OptionRegistry) {
            $($use::init_config(registry);)*
            $($(
                let name = $crate::options!(@name $option $($option_name)?);
                let docs = concat!("" $(,$option_desc,)" "*);
                $crate::options!(@register registry name docs $default, $ty $(,$option_validator)?);
            )+)+
        }
        $(impl $ident for $crate::OptionManager {
            $(
                $(#[doc = $option_desc])*
                fn $option(&self) -> $crate::options!(@ret_ty $($($extra)*)? $ty) {
                    let name = $crate::options!(@name $option $($option_name)?);
                    $crate::options!(@get $($($extra)*)? self, $ty, name)
                }
            )+
        })+
    };
    (@register $registry: ident $name: ident $desc: ident $default: expr, $ty:ty) => {{
        use $crate::IntoTy;
        let val: $ty = $default.into_ty();
        $registry.register($name, $desc, val);
    }};
    (@register $registry: ident $name: ident $desc: ident  $default: expr, $ty:ty, $validator: expr) => {{
        use $crate::IntoTy;
        let val: $ty = $default.into_ty();
        $registry.register_with_validator($name, $desc, val, $validator);
    }};
    (@name $ident: ident) => {
        ::std::stringify!($ident)
    };
    (@name $ident: ident $name: literal) => {
        $name
    };
    (@ret_ty copy $ty: ty) => {
        $ty
    };
    (@ret_ty map($fn: expr, $ret_ty: ty) $ty: ty) => {
        $ret_ty
    };
    (@ret_ty fold($init: expr, $fn: expr, $ret_ty: ty) $ty: ty) => {
        $ret_ty
    };
    (@ret_ty deref $ty: ty) => {
        $crate::Guard<'_, <$ty as ::std::ops::Deref>::Target>
    };
    (@ret_ty $ty: ty) => {
        $crate::Guard<'_, $ty>
    };
    (@get map($fn: expr, $ret_ty: ty) $config: ident, $ty: ty, $name: ident) => {
        let val = $config.get::<$ty>($name);
        $fn(val)
    };
    (@get fold($init: expr, $fn: expr, $ret_ty: ty) $config: ident, $ty: ty, $name: ident) => {
        $config.get_folded::<$ty, $ret_ty>($name, $init, $fn)
    };
    (@get copy $config: ident, $ty: ty, $name: ident) => {
        *$config.get::<$ty>($name)
    };
    (@get deref $config: ident, $ty: ty, $name: ident) => {
        $config.get_deref::<$ty>($name)
    };
    (@get $config: ident, $ty: ty, $name: ident) => {
        $config.get::<$ty>($name)
    };
}

#[macro_export]
macro_rules! config_serde_adapter {
    ($ty: ident) => {
        impl $crate::Ty for $ty {
            fn to_value(&self) -> $crate::Value {
                $crate::to_value(self).unwrap()
            }
            fn from_value(val: $crate::Value) -> ::anyhow::Result<Self> {
                let val = $crate::from_value(val)?;
                Ok(val)
            }
        }
    };
}
