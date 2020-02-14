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

function valueFromKeyPath(obj, keyPath) {
  keyPath = keyPath.split('.');
  return keyPath.slice(1)
    .reduce((acc, k) => acc[k], obj[keyPath[0]]);
}

function setValueFromKeyPath(obj, keyPath, val) {
  keyPath = keyPath.split('.');
  lastKey = keyPath.pop();
  if (keyPath.length > 0) {
    let curr = obj[keyPath[0]];
    keyPath.slice(1).forEach((k) => {
      curr = curr[k];
    });
    curr[lastKey] = val;
  } else {
    obj[lastKey] = val;
  }
}
