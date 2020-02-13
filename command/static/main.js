const history = [];

const CONFIG_SPEC = {
  'POPULATION': {
    desc: 'The number of agents to create. Higher numbers will run slower but can provide better results.'
  },
  'SEED': {
    desc: 'Use a consistent seed value to control for randomness across runs. You probably don\'t need to change this.'
  }
};
let currentConfig = {};

function loadConfig() {
  get('/config', {}, ({config}) => {
    let changed = new Set();
    let el = document.querySelector('#config ul');
    let resetButton = document.getElementById('reset--new-config');
    el.innerHTML = '';
    resetButton.style.display = 'none';
    Object.keys(CONFIG_SPEC).forEach((k) => {
      let val = config[k];
      let spec = CONFIG_SPEC[k];
      currentConfig[k] = val;
      let html = `<li class="config-item">
        <div class="config-item--info">
          <div class="config-item--key">${k}</div>
          <div class="config-item--val">${val}</div>
          <input class="config-item--input" type="text" value="${val}">
        </div>
        <div class="config-item--desc">${spec.desc}</div>
      </li>`;

      // Setup editable inputs
      let child = htmlToElement(html);
      let valEl = child.querySelector('.config-item--val');
      let inputEl = child.querySelector('input');
      valEl.addEventListener('click', () => {
        inputEl.style.display = 'block';
        valEl.style.display = 'none';
        inputEl.select();
      });
      inputEl.addEventListener('blur', () => {
        let customVal = parseInt(inputEl.value);

        // Reset if not valid number
        if (isNaN(customVal)) {
          inputEl.value = val;
          customVal = val;
        }

        // Show original value if changed
        if (customVal !== val) {
          valEl.innerHTML = `
            <span class="config-item--val-original">${val}</span>
            <span class="config-item--val-new">${customVal}</span>`;
          changed.add(k);
        } else {
          valEl.innerText = val;
          changed.delete(k);
        }
        inputEl.style.display = 'none';
        valEl.style.display = 'block';
        currentConfig[k] = customVal;

        if (changed.size > 0) {
          resetButton.style.display = 'block';
        } else {
          resetButton.style.display = 'none';
        }
      });
      el.appendChild(child);
    });
  });
}

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
          updateCharts(data.history);
          queryingState = false;
        });
      } else {
          queryingState = false;
      }
    });
  }
}

function waitForReady(cb) {
  let interval = setInterval(() => {
    updateStatus((status) => {
      if (status == 'ready') {
        clearInterval(interval);
        if (cb) cb();
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

function reset() {
  post('/reset', currentConfig, () => {
    waitForReady(() => {
      loadConfig();
      resetCharts();
      history.length = 0; // Clear history
    });
  });
}
document.querySelector('#reset button').addEventListener('click', reset);
document.getElementById('reset--new-config').addEventListener('click', reset);

updateStatus();
loadConfig();

// TODO what if we miss a state update?
// i.e. multiple steps occur during this interval
setInterval(updateState, 500);

const CHARTS = [{
  title: 'Shares per Content',
  datasets: [{
    label: 'max',
    key: 'shares.max'
  }, {
    label: 'min',
    key: 'shares.min'
  }, {
    label: 'mean',
    key: 'shares.mean'
  }]
}, {
  title: 'Followers',
  datasets: [{
    label: 'max',
    key: 'followers.max'
  }, {
    label: 'min',
    key: 'followers.min'
  }, {
    label: 'mean',
    key: 'followers.mean'
  }]
}];

const COLORS = [
  'rgb(  0, 153, 102)',
  'rgb(230,   0,   0)',
  'rgb( 26, 102, 230)',
  'rgb(102,  26, 230)',
  'rgb(230,  77,   0)'
];

Chart.defaults.scale.ticks.fontSize = 9;
Chart.defaults.scale.ticks.fontFamily = 'monospace';
function createChart(spec) {
  let chartEl = document.createElement('canvas');
  let chart = new Chart(chartEl, {
    type: 'line',
    data: {
      labels: [],
      datasets: spec.datasets.map((d, i) => ({
        label: d.label,
        fill: false,
        borderWidth: 1,
        pointRadius: 0,
        backgroundColor: COLORS[i],
        borderColor: COLORS[i],
        data: []
      }))
    },
    options: {
      animation: {
        duration: 0
      },
      legend: {
        display: true,
        labels: {
          boxWidth: 2,
          fontSize: 11,
          fontFamily: 'monospace'
        }
      },
      scales: {
        yAxes: [{
          ticks: {
            min: 0
          },
          display: true
        }],
        xAxes: [{
          display: true
        }]
      }
    }
  });
  return {chart, chartEl};
}

const stage = document.getElementById('charts');
CHARTS.forEach((c) => {
  let {chart, chartEl} = createChart(c);
  c.chart = chart;

  titleEl = document.createElement('div');
  titleEl.innerText = c.title;
  titleEl.className = 'chart-title';

  parentEl = document.createElement('div');
  parentEl.className = 'chart';
  parentEl.appendChild(titleEl);
  parentEl.appendChild(chartEl);
  stage.appendChild(parentEl);
});

function updateCharts(newStates) {
  CHARTS.forEach((c) => {
    newStates.forEach((s) => {
      c.chart.data.labels.push(s.step);
    });
    c.chart.data.datasets.forEach((dataset, i) => {
      let spec = c.datasets[i];
      let keyPath = spec.key.split('.');
      newStates.forEach((s) => {
        let value = keyPath.slice(1).reduce((acc, k) => acc[k], s[keyPath[0]]);
        dataset.data.push(value);
      });
    });
    c.chart.update();
  });
}

function resetCharts() {
  CHARTS.forEach((c) => {
    c.chart.data.labels = [];
    c.chart.data.datasets.forEach((dataset, i) => {
      dataset.data = [];
    });
    c.chart.update();
  });
}
