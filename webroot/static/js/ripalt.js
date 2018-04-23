const ds_prefixes = ["", "Ki", "Mi", "Gi", "Ti", "Pi", "Ei", "Zi", "Yi"];

function data_size(amount) {
    let was_negative = false;
    if (amount < 0) {
        amount -= amount;
        was_negative = true;
    }
    let prefix = 0;

    while (amount >= 1024 && prefix < 8) {
        amount = amount / 1024;
        prefix += 1;
    }

    if (was_negative) {
        amount = -amount;
    }

    if (prefix === 0) {
        return amount.toFixed(0) + " B";
    }

    return amount.toFixed(2) + " " + ds_prefixes[prefix] + "B";
}