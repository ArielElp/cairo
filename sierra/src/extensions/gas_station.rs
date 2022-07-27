use crate::{
    extensions::*,
    utils::{as_deferred, gas_builtin_type},
};

struct GetGasExtension {}

impl ExtensionImplementation for GetGasExtension {
    fn get_signature(
        self: &Self,
        tmpl_args: &Vec<TemplateArg>,
    ) -> Result<ExtensionSignature, Error> {
        single_value_arg(tmpl_args)?;
        Ok(ExtensionSignature {
            args: vec![gas_builtin_type()],
            results: vec![
                vec![as_deferred(gas_builtin_type())],
                vec![gas_builtin_type()],
            ],
            fallthrough: Some(1),
        })
    }

    fn ref_values(
        self: &Self,
        tmpl_args: &Vec<TemplateArg>,
        _registry: &TypeRegistry,
        _cursors: &Cursors,
        arg_refs: Vec<RefValue>,
    ) -> Result<Vec<Vec<RefValue>>, Error> {
        Ok(vec![
            vec![RefValue::OpWithConst(
                as_final(&arg_refs[0])?,
                Op::Sub,
                gas_value_arg(tmpl_args)?,
            )],
            vec![arg_refs[0].clone()],
        ])
    }

    fn effects(
        self: &Self,
        tmpl_args: &Vec<TemplateArg>,
        _registry: &TypeRegistry,
    ) -> Result<Vec<Effects>, Error> {
        Ok(vec![
            gas_usage(-gas_value_arg(tmpl_args)? + 1),
            gas_usage(1),
        ])
    }

    fn exec(
        self: &Self,
        tmpl_args: &Vec<TemplateArg>,
        _registry: &TypeRegistry,
        mut inputs: Vec<Vec<i64>>,
    ) -> Result<(Vec<Vec<i64>>, usize), Error> {
        let gas = gas_value_arg(tmpl_args)?;
        validate_mem_sizes(&inputs, [1])?;
        if inputs[0][0] >= gas {
            Ok((vec![vec![inputs[0][0] - gas]], 0))
        } else {
            Ok((vec![inputs.remove(0)], 1))
        }
    }
}

struct RefundGasExtension {}

impl NonBranchImplementation for RefundGasExtension {
    fn get_signature(
        self: &Self,
        tmpl_args: &Vec<TemplateArg>,
    ) -> Result<(Vec<Type>, Vec<Type>), Error> {
        single_value_arg(tmpl_args)?;
        Ok((
            vec![gas_builtin_type()],
            vec![as_deferred(gas_builtin_type())],
        ))
    }

    fn ref_values(
        self: &Self,
        tmpl_args: &Vec<TemplateArg>,
        _registry: &TypeRegistry,
        _cursors: &Cursors,
        arg_refs: Vec<RefValue>,
    ) -> Result<Vec<RefValue>, Error> {
        Ok(vec![RefValue::OpWithConst(
            as_final(&arg_refs[0])?,
            Op::Add,
            gas_value_arg(tmpl_args)?,
        )])
    }

    fn effects(
        self: &Self,
        tmpl_args: &Vec<TemplateArg>,
        _registry: &TypeRegistry,
    ) -> Result<Effects, Error> {
        Ok(gas_usage(gas_value_arg(tmpl_args)?))
    }

    fn exec(
        self: &Self,
        tmpl_args: &Vec<TemplateArg>,
        _registry: &TypeRegistry,
        inputs: Vec<Vec<i64>>,
    ) -> Result<Vec<Vec<i64>>, Error> {
        validate_mem_sizes(&inputs, [1])?;
        Ok(vec![vec![inputs[0][0] + gas_value_arg(tmpl_args)?]])
    }
}

fn gas_value_arg(tmpl_args: &Vec<TemplateArg>) -> Result<i64, Error> {
    let gas = single_value_arg(tmpl_args)?;
    if gas <= 0 {
        Err(Error::UnsupportedTypeArg)
    } else {
        Ok(gas)
    }
}

pub(super) fn extensions() -> [(String, ExtensionBox); 2] {
    [
        ("get_gas".to_string(), Box::new(GetGasExtension {})),
        (
            "refund_gas".to_string(),
            wrap_non_branch(Box::new(RefundGasExtension {})),
        ),
    ]
}

struct GasBuiltinTypeInfo {}

impl TypeInfoImplementation for GasBuiltinTypeInfo {
    fn get_info(
        self: &Self,
        tmpl_args: &Vec<TemplateArg>,
        _: &TypeRegistry,
    ) -> Result<TypeInfo, Error> {
        validate_size_eq(tmpl_args, 0)?;
        Ok(TypeInfo { size: 1 })
    }
}

pub(super) fn types() -> [(String, TypeInfoBox); 1] {
    [(gas_builtin_type().name, Box::new(GasBuiltinTypeInfo {}))]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::{as_type, type_arg, val_arg};

    #[test]
    fn legal_usage() {
        assert_eq!(
            RefundGasExtension {}.get_signature(&vec![val_arg(5)]),
            Ok((
                vec![gas_builtin_type()],
                vec![as_deferred(gas_builtin_type())],
            ))
        );
    }

    #[test]
    fn wrong_num_of_args() {
        assert_eq!(
            GetGasExtension {}.get_signature(&vec![val_arg(1), val_arg(2)]),
            Err(Error::WrongNumberOfTypeArgs)
        );
        assert_eq!(
            GetGasExtension {}.get_signature(&vec![]),
            Err(Error::WrongNumberOfTypeArgs)
        );
        assert_eq!(
            RefundGasExtension {}.get_signature(&vec![]),
            Err(Error::WrongNumberOfTypeArgs)
        );
    }

    #[test]
    fn wrong_arg_type() {
        assert_eq!(
            GetGasExtension {}.get_signature(&vec![type_arg(as_type("1"))]),
            Err(Error::UnsupportedTypeArg)
        );
        assert_eq!(
            RefundGasExtension {}.get_signature(&vec![type_arg(as_type("1"))]),
            Err(Error::UnsupportedTypeArg)
        );
    }
}
