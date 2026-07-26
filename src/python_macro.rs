macro_rules! _wrap_pyclass {
    (
        $name:ident($inner:ty),
        name = $python_name:literal
        $(, super = $super:ty)?
        $(, attrs = [$($attr:ident),* $(,)?])? $(,)?
    ) => {
        #[pyclass(
            $(extends = $super,)?
            $($($attr,)*)?
            name = $python_name
        )]
        pub struct $name($inner);
    };

    (
        $name:ident {
            $($field:ident: $field_type:ty),+ $(,)?
        },
        name = $python_name:literal
        $(, super = $super:ty)?
        $(, attrs = [$($attr:ident),* $(,)?])? $(,)?
    ) => {
        #[pyclass(
            $(extends = $super,)?
            $($($attr,)*)?
            name = $python_name
        )]
        pub struct $name {
            $(
                $field: $field_type,
            )+
        }
    };

    (
        $name:ident,
        name = $python_name:literal
        $(, super = $super:ty)?
        $(, attrs = [$($attr:ident),* $(,)?])? $(,)?
    ) => {
        #[pyclass(
            $(extends = $super,)?
            $($($attr,)*)?
            name = $python_name
        )]
        pub struct $name;
    };
}

macro_rules! py_class {
    ($($class:tt),+ $(,)?) => {
        $(
            _wrap_pyclass! $class;
        )+
    };
}

macro_rules! _wrap_pyenum {
    (
        $py_name:ident,
        $rust_name:ty,
        $python_name:literal,
        [$($variant:ident),+ $(,)?]
    ) => {
        #[pyclass(eq, skip_from_py_object, name = $python_name)]
        #[derive(Clone, Copy, PartialEq)]
        pub enum $py_name {
            $(
                $variant,
            )+
        }

        impl From<$rust_name> for $py_name {
            fn from(value: $rust_name) -> Self {
                match value {
                    $(
                        <$rust_name>::$variant => Self::$variant,
                    )+
                }
            }
        }
    };
}

macro_rules! py_enum {
    ($($enum:tt),+ $(,)?) => {
        $(
            _wrap_pyenum! $enum;
        )+
    };
}

macro_rules! build_pyconstant_instance {
    ($py:expr, $child:expr) => {{
        Bound::new(
            $py,
            PyClassInitializer::from(PyExpression {})
                .add_subclass(PyConstant {})
                .add_subclass($child),
        )?
        .into_super()
        .into_super()
        .unbind()
    }};
}

macro_rules! build_pyexpression_instance {
    ($py:expr, $child:expr) => {{
        Bound::new(
            $py,
            PyClassInitializer::from(PyExpression {}).add_subclass($child),
        )?
        .into_super()
        .unbind()
    }};
}

macro_rules! _wrap_pygetters {
    (
        $type:ty,
        {
            $(
                $getter:ident: $return_type:ty,
                |$this:tt, $py:ident| $body:block
            ),+ $(,)?
        }
    ) => {
        #[pymethods]
        impl $type {
            $(
                #[getter]
                fn $getter(
                    &$this,
                    $py: Python<'_>,
                ) -> $return_type $body
            )+
        }
    };
}

macro_rules! py_getters {
    ($($getter_group:tt),+ $(,)?) => {
        $(
            _wrap_pygetters! $getter_group;
        )+
    };
}
