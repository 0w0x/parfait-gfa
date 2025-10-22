#[macro_export]
macro_rules! impl_enum_find_accessors {
    (
        for $ty:ty;
        $( $variant:ident => $Type:ident ),+ $(,)?
    ) => {
        impl $ty {
            $(
                paste::paste! {
                    #[inline]
                    pub fn [<find_ $variant>](&self, line_no: usize) -> Option<&$Type> {
                        let idx = *self.records_index.get(&line_no)?;
                        self.records.get(idx).and_then(GfaRecord::[<as_ $variant>])
                    }

                    #[inline]
                    pub fn [<find_ $variant _mut>](&mut self, line_no: usize) -> Option<&mut $Type> {
                        let idx = *self.records_index.get(&line_no)?;
                        self.records.get_mut(idx).and_then(GfaRecord::[<as_mut_ $variant>])
                    }
                }
            )+
        }
    };
}

#[macro_export]
macro_rules! record_accessors {
    (impl $Enum:ident {
        $(
            $Variant:ident ( $Ty:ty ) => ( $as:ident, $as_mut:ident );
        )*
    }) => {
        impl $Enum {
            $(
                #[inline]
                pub fn $as(&self) -> Option<&$Ty> {
                    if let Self::$Variant(x) = self { Some(x) } else { None }
                }

                #[inline]
                pub fn $as_mut(&mut self) -> Option<&mut $Ty> {
                    if let Self::$Variant(x) = self { Some(x) } else { None }
                }
            )*
        }
    };
}

#[macro_export]
macro_rules! parse_case {
    ($Type:ty, $Variant:ident, $args:expr) => {{
        let (opt, errs) = <$Type>::parse_line($args);
        (opt.map(GfaRecord::$Variant), errs)
    }};
}
