function toParams(obj) {
    return Object
        .keys(obj)
        .map(k => `${encodeURIComponent(k)}=${encodeURIComponent(obj[k])}`)
        .join('&');
}

function get(url, data, onSuccess, onErr) {
  url = `${url}?${toParams(data)}`;
  fetch(url, {
    headers: {
      'Accept': 'application/json',
      'Content-Type': 'application/json'
    },
    credentials: 'same-origin',
    method: 'GET',
  })
    .then(res => res.json())
    .then((data) => onSuccess && onSuccess(data))
    .catch(err => { console.log(err) });
}

function post(url, data, onSuccess, onErr) {
  fetch(url, {
    headers: {
      'Accept': 'application/json',
      'Content-Type': 'application/json'
    },
    credentials: 'same-origin',
    method: 'POST',
    body: JSON.stringify(data)
  })
    .then(res => res.json())
    .then((data) => onSuccess && onSuccess(data))
    .catch(err => { throw err });
}

function htmlToElement(html) {
  let template = document.createElement('template');
  html = html.trim();
  template.innerHTML = html;
  return template.content.firstChild;
}
