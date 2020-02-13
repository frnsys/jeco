const CONFIG_SPEC = {
  'POPULATION': {
    desc: 'The number of agents to create. Higher numbers will run slower but can provide better results.'
  },
  'SEED': {
    desc: 'Use a consistent seed value to control for randomness across runs. You probably don\'t need to change this.'
  }
};

class Command {
  constructor(elements, plotter) {
    this.config = {};
    this.loadConfig();

    this.history = [];
    this.plotter = plotter;
    this.elements = Object.keys(elements).reduce((acc, k) => {
      acc[k] = document.querySelector(elements[k]);
      return acc;
    }, {});

    this.listeners = {
      ready: [],
      loading: [],
      running: []
    };

    this.elements['step']
      .addEventListener('click', () => {
        let steps = parseInt(this.elements['stepInput'].value);
        this.runSimulation(steps)
      });
    this.elements['reset']
      .addEventListener('click', this.reset.bind(this));
    this.elements['configReset']
      .addEventListener('click', this.reset.bind(this));

    this.queryingState = false;
    setInterval(() => {
      this.queryStatus();
      this.queryState();
    }, 500);
  }

  queryState() {
    if (!this.queryingState) {
      this.queryingState = true;
      get('/state/step', {}, ({step}) => {
        // Query full state only if it's new
        if (step >= this.history.length) {
          get('/state/history', {
            from: this.history.length,
            to: step
          }, (data) => {
            console.log(data.history);
            data.history.forEach((s) => this.history.push(s));
            this.plotter.append(data.history);
            this.queryingState = false;
          });
        } else {
          this.queryingState = false;
        }
      });
    }
  }

  queryStatus() {
    let el = this.elements['status'];
    get('/status', {}, ({status}) => {
      let stepEnabled = false;
      switch (status) {
        case 'ready':
          stepEnabled = true;
          el.style.background = '#009966';
          break;
        case 'running':
          el.style.background = '#7b16c1';
          break;
        case 'loading':
          el.style.background = '#E50000';
          break;
      }
      el.innerText = status.toUpperCase();
      this.elements['step'].disabled = !stepEnabled;

      // Trigger status event listeners
      this.listeners[status].forEach((fn) => fn());
      this.listeners[status] = [];
    });
  }

  runSimulation(steps) {
    post('/step', {
      steps: steps
    }, () => {});
  }

  on(status, fn) {
    this.listeners[status].push(fn);
  }

  reset() {
    post('/reset', this.config, () => {
      this.on('ready', () => {
        this.loadConfig();
        this.history = [];
        this.plotter.reset();
      });
    });
  }

  loadConfig() {
    get('/config', {}, ({config}) => {
      // Keep track of when the config is dirty
      let changed = new Set();

      let el = this.elements['config'];
      el.innerHTML = '';

      let resetButton = this.elements['configReset']
      resetButton.style.display = 'none';

      // Display only config items specified
      // in the spec
      Object.keys(CONFIG_SPEC).forEach((k) => {
        let val = config[k];
        let spec = CONFIG_SPEC[k];
        this.config[k] = val;

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
          this.config[k] = customVal;

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
}
