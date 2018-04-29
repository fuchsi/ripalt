'use strict';
const ds_prefixes = ['', 'Ki', 'Mi', 'Gi', 'Ti', 'Pi', 'Ei', 'Zi', 'Yi'];

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
        return amount.toFixed(0) + ' B';
    }

    return amount.toFixed(2) + ' ' + ds_prefixes[prefix] + 'B';
}

function get_json(url) {
    return fetch(url, {credentials: 'same-origin'})
        .then((resp) => {
            if (resp.ok) {
                return resp.json();
            }
            throw new Error('Network response was not ok.');
        })
        .catch(error => console.error('Error:', error));
}

function post_json(url, data) {
    return fetch(url, {
        credentials: 'same-origin',
        method: 'post',
        body: JSON.stringify(data),
        headers: {'Content-Type': 'application/json'}
    }).then((resp) => {
        if (resp.ok) {
            return resp.json();
        }
        throw new Error('Network response was not ok.');
    }).catch(error => console.error('Error:', error));
}

function shoutbox_add_line(target, data) {
    if ($(`li#cm-${data.id}`).length !== 0) {
        return;
    }
    const options = {hour: '2-digit', minute: '2-digit', second: '2-digit'};
    const date = new Date(data.created_at);
    let line = $(`<li id="cm-${data.id}" class="shoutbox-line">`)
        .append($(`<span class="shoutbox-date">[${date.toLocaleString('de-DE', options)}]</span>`))
        .append(' ')
        .append($(`<span class="shoutbox-user">&lt;<a class="user-group-${data.user_group}" href="/user/${data.user_id}">${data.user_name}</a>&gt;</span>`))
        .append(' ')
        .append($('<span class="shoutbox-message">').html(inline_markdown(data.message)));

    target.append(line);
}

function inline_markdown(text) {
    text = text.replace(/</g, '&lt;').replace(/>/g, '&gt;');
    text = text.replace(/(\*\*|__)(.+?)\1/g, '<strong>$2</strong>');
    text = text.replace(/(\*|_)(.+?)\1/g, '<em>$2</em>');
    text = text.replace(/`(.+?)`/g, '<code>$1</code>');
    text = text.replace(/\[([^\]]+)\]\(([^)]+)\)/g, (match, name, url) => {
        let real_url;
        try {
            let base = window.location;
            real_url = new URL(url, base);
        } catch (err) {
            console.log('Error parsing url: ', err);
            return match;
        }
        return `<a href="${real_url.href}" target="_blank">${name}</a>`;
    });

    return text;
}

function update_stats() {
    get_json('/api/v1/user/stats')
        .then(data => {
            $('#navbarDropdownUserMenuLink').text(data.name);
            $('#navbar-downloads').text(data.downloads);
            $('#navbar-downloaded').text(data_size(data.downloaded));
            $('#navbar-uploads').text(data.uploads);
            $('#navbar-uploaded').text(data_size(data.uploaded));
            $('#navbar-ratio').text(data.ratio.toFixed(3));
        });
}

function update_chatrooms() {
    if (chatrooms === undefined) {
        return;
    }
    for (let i = 0; i < chatrooms.length; i++) {
        let chat = chatrooms[i];
        let target = $(`#shoutbox-${chat.id}>ul`);
        let url = `/api/v1/chat/messages?chat=${chat.nid}`;
        let first_run = true;
        if (chat.last_update !== undefined) {
            url += `&since=${chat.last_update}`;
            first_run = false;
        }
        get_json(url)
            .then((data) => {
                if (data === undefined) {
                    return;
                }
                chatrooms[i].last_update = (Date.now() / 1000).toFixed(0);
                if (!first_run && data.length > 0) {
                    let badge = $(`#${chat.id}-tab:not([class*=active]) span.badge`);
                    if (badge.length === 1) {
                        let new_message = data.length;
                        if (badge.text() !== '') {
                            try {
                                console.log(parseInt(badge.text(), 10));
                                new_message += parseInt(badge.text(), 10);
                            } catch (e) {
                                console.log(e);
                            }
                        }
                        badge.text(new_message.toFixed(0)).removeClass('invisible');
                        $(`#${chat.id}-tab:not([class*=active])`).one('click', (ev) => {
                            $('span.badge', ev.target).addClass('invisible');

                        });
                    }
                }
                if (data.length > 0) {
                    data.reverse().forEach(message => {
                        shoutbox_add_line(target, message);
                    });
                    let box_target = target.parent();
                    if (box_target.height() < box_target.prop('scrollHeight')) {
                        let scroll_top = box_target.prop('scrollHeight') - box_target.height();
                        box_target.prop('scrollTop', scroll_top);
                    }
                }
            })
            .catch((error) => console.error('Error:', error));
    }
}
