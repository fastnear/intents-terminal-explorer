use near_gas::NearGas;
use near_token::NearToken;

/// Format gas amount in human-readable format
/// Examples: "30 TGas", "5 GGas", "100 Gas"
pub fn format_gas(gas: u64) -> String {
    let near_gas = NearGas::from_gas(gas);
    format!("{}", near_gas)
}

/// Format NEAR token amount in human-readable format
/// Examples: "1 NEAR", "0.5 NEAR", "1000000 yoctoNEAR"
pub fn format_near(yoctonear: u128) -> String {
    let token = NearToken::from_yoctonear(yoctonear);
    format!("{}", token)
}

/// Format gas with compact suffix for UI (e.g., "30T" instead of "30 TGas")
#[allow(dead_code)]
pub fn format_gas_compact(gas: u64) -> String {
    if gas == 0 {
        return "0".to_string();
    }
    const TERA: u64 = 1_000_000_000_000;
    const GIGA: u64 = 1_000_000_000;
    const MEGA: u64 = 1_000_000;

    if gas >= TERA {
        format!("{}T", gas / TERA)
    } else if gas >= GIGA {
        format!("{}G", gas / GIGA)
    } else if gas >= MEGA {
        format!("{}M", gas / MEGA)
    } else {
        gas.to_string()
    }
}

/// Format NEAR amount with compact suffix for UI (e.g., "1.5Ⓝ")
#[allow(dead_code)]
pub fn format_near_compact(yoctonear: u128) -> String {
    if yoctonear == 0 {
        return "0Ⓝ".to_string();
    }
    const NEAR: u128 = 1_000_000_000_000_000_000_000_000;

    if yoctonear >= NEAR {
        let near_amount = yoctonear / NEAR;
        let remainder = yoctonear % NEAR;
        if remainder == 0 {
            format!("{}Ⓝ", near_amount)
        } else {
            // Show 1 decimal place
            let decimal = (remainder * 10) / NEAR;
            format!("{}.{}Ⓝ", near_amount, decimal)
        }
    } else {
        // Sub-NEAR amounts
        format!("{}y", yoctonear)
    }
}
