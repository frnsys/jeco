const history = [];

function updateStatus(cb) {
  let el = document.getElementById('status');
  get('/status', {}, ({status}) => {
    el.innerText = status.toUpperCase();
    switch (status) {
      case 'ready':
        el.style.background = 'green';
        break;
      case 'running':
        el.style.background = 'yellow';
        break;
      case 'loading':
        el.style.background = 'red';
        break;
    }
    if (cb) cb(status);
  });
}

let queryingState = false;

function updateState() {
  if (!queryingState) {
    queryingState = true;
    get('/state/step', {}, ({step}) => {
      // Query full state only if it's new
      if (step >= history.length) {
        get('/state/history', {
          from: history.length,
          to: step
        }, (data) => {
          console.log(data.history);
          data.history.forEach((s) => history.push(s));
          queryingState = false;
        });
      }
    });
  }
}

function waitForReady() {
  let interval = setInterval(() => {
    updateStatus((status) => {
      if (status == 'ready') {
        clearInterval(interval);
      }
    });
  }, 500);
}

document.querySelector('#step button').addEventListener('click', () => {
  updateStatus((status) => {
    if (status == 'ready') {
      let steps = document.querySelector('#step input').value;
      post('/step', {
        steps: parseInt(steps)
      }, () => {
        // pass
        waitForReady();
      });
    }
  });
});

document.querySelector('#reset button').addEventListener('click', () => {
  post('/reset', {}, () => {
    waitForReady();
    history.length = 0; // Clear history
  });
});

updateStatus();

// TODO what if we miss a state update?
// i.e. multiple steps occur during this interval
setInterval(updateState, 500);
