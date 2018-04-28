const ds_prefixes = ["", "Ki", "Mi", "Gi", "Ti", "Pi", "Ei", "Zi", "Yi"];

function data_size(amount) {
    let was_negative = false;
    if (amount < 0) {
        amount = -amount;
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

function get_json(url) {
    return fetch(url, {credentials: 'same-origin'}).then(resp => resp.json())
}

function post_json(url, data) {
    return fetch(url, {
        credentials: 'same-origin',
        method: 'post',
        body: JSON.stringify(data),
        headers: {'Content-Type': 'application/json'}
    }).then(resp => resp.json())
}

function shoutbox_add_line(target, data) {
    if ($(`li#cm-${data.id}`).length !== 0) {
        return;
    }
    const options = {timeZone: 'UTC', hour: '2-digit', minute: '2-digit', second: '2-digit' };
    const date = new Date(data.created_at);
    let line = $(`<li id="cm-${data.id}" class="shoutbox-line">`)
        .append($(`<span class="shoutbox-date">[${date.toLocaleString('de-DE', options)}]</span>`))
        .append(' ')
        .append($(`<span class="shoutbox-user">&lt;<a href="/user/${data.user_id}">${data.user_name}</a>&gt;</span>`))
        .append(' ')
        .append($('<span class="shoutbox-message">').html(inline_markdown(data.message)));

    target.append(line);
}

function inline_markdown(text) {
    text = text.replace(/</g, '&lt;').replace(/>/g, '&gt;');
    text = text.replace(/(\*\*|__)(.+?)\1/g, "<strong>$2</strong>");
    text = text.replace(/(\*|_)(.+?)\1/g, "<em>$2</em>");
    text = text.replace(/`(.+?)`/g, "<code>$1</code>");
    text = text.replace(/\[([^\]]+)\]\(([^)]+)\)/g, (match, name, url) => {
        let real_url;
        try {
            let base = window.location
            real_url = new URL(url, base);
        } catch (err) {
            console.log("Error parsing url: ", err);
            return match;
        }
        return `<a href="${real_url.href}" target="_blank">${name}</a>`;
    });

    return text;
}