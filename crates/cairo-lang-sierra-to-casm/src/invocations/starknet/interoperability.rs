use cairo_lang_casm::builder::CasmBuilder;
use cairo_lang_casm::casm_build_extend;
use cairo_lang_casm::operand::ResOperand;
use cairo_lang_sierra::extensions::consts::SignatureAndConstConcreteLibfunc;
use num_bigint::BigInt;
use num_traits::{One, Zero};

use super::{CompiledInvocation, CompiledInvocationBuilder, InvocationError};
use crate::invocations::get_non_fallthrough_statement_id;
use crate::references::{CellExpression, ReferenceExpression};

#[cfg(test)]
#[path = "interoperability_test.rs"]
mod test;

/// Builds instructions for StarkNet call contract system call.
pub fn build_call_contract(
    builder: CompiledInvocationBuilder<'_>,
) -> Result<CompiledInvocation, InvocationError> {
    let failure_handle_statement_id = get_non_fallthrough_statement_id(&builder);
    let selector_imm = BigInt::from_bytes_le(num_bigint::Sign::Plus, "call_contract".as_bytes());

    let [expr_gas_builtin, expr_system, expr_address, expr_arr] = builder.try_get_refs()?;
    let gas_builtin = expr_gas_builtin.try_unpack_single()?.to_deref()?;
    let system = expr_system.try_unpack_single()?.to_buffer(8)?;
    let contract_address = expr_address.try_unpack_single()?.to_deref()?;
    let [call_data_start, call_data_end] = expr_arr.try_unpack()?;
    let call_data_start = call_data_start.to_deref()?;
    let call_data_end = call_data_end.to_deref()?;

    let mut casm_builder = CasmBuilder::default();
    let system = casm_builder.add_var(system);
    let gas_builtin = casm_builder.add_var(ResOperand::Deref(gas_builtin));
    let contract_address = casm_builder.add_var(ResOperand::Deref(contract_address));
    let call_data_start = casm_builder.add_var(ResOperand::Deref(call_data_start));
    let call_data_end = casm_builder.add_var(ResOperand::Deref(call_data_end));
    casm_build_extend! {casm_builder,
        const selector_imm = selector_imm;
        tempvar selector = selector_imm;
        let original_system = system;
        assert selector = *(system++);
        assert gas_builtin = *(system++);
        assert contract_address = *(system++);
        assert call_data_start = *(system++);
        assert call_data_end = *(system++);
        hint SystemCall { system: original_system };

        let updated_gas_builtin = *(system++);
        // `revert_reason` is 0 on success, nonzero on failure/revert.
        tempvar revert_reason = *(system++);
        let res_start = *(system++);
        let res_end = *(system++);
        jump Failure if revert_reason != 0;
    };
    Ok(builder.build_from_casm_builder(
        casm_builder,
        [
            ("Fallthrough", &[&[updated_gas_builtin], &[system], &[res_start, res_end]], None),
            (
                "Failure",
                &[&[updated_gas_builtin], &[system], &[revert_reason], &[res_start, res_end]],
                Some(failure_handle_statement_id),
            ),
        ],
    ))
}

/// Handles the storage_address_const libfunc.
pub fn build_contract_address_const(
    builder: CompiledInvocationBuilder<'_>,
    libfunc: &SignatureAndConstConcreteLibfunc,
) -> Result<CompiledInvocation, InvocationError> {
    let addr_bound: BigInt = (BigInt::one() << 251) - 256;
    let addr = &libfunc.c;
    if addr.is_zero() || addr >= &addr_bound {
        return Err(InvocationError::InvalidGenericArg);
    }

    Ok(builder.build_only_reference_changes(
        [ReferenceExpression::from_cell(CellExpression::Immediate(libfunc.c.clone()))].into_iter(),
    ))
}
