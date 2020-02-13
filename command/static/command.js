const CONFIG_SPEC = {
  'POPULATION': {
    type: 'int',
    desc: 'The number of agents to create. Higher numbers will run slower but can provide better results.'
  },
  'SEED': {
    type: 'int',
    desc: 'Use a consistent seed value to control for randomness across runs. You probably don\'t need to change this.'
  },
  'MAX_INFLUENCE': {
    type: 'float',
    desc: 'Maximum amount a piece of content can influence a person\'s values.'
  },
  'GRAVITY_STRETCH': {
    type: 'float',
    desc: 'Horizontal stretching of gravity function. Higher values mean weaker influence at greater distances.'
  }
};

const POLICY_SPEC = {
  'PopulationChange': {
    desc: 'Change the population by the specified amount.',
    args: [{
      min: 0,
      type: 'int',
      name: 'amount',
      default: 100
    }]
  }
};

class Command {
  constructor(elements, plotter) {
    this.config = {};

    this.history = [];
    this.policies = [];
    this.plotter = plotter;
    this.elements = Object.keys(elements).reduce((acc, k) => {
      acc[k] = document.querySelector(elements[k]);
      return acc;
    }, {});

    this.loadConfig();
    this.loadPolicies();

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
      let enabled = false;
      switch (status) {
        case 'ready':
          enabled = true;
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
      this.elements['step'].disabled = !enabled;
      [...this.elements['policies'].querySelectorAll('button')]
        .forEach((b) => {
          b.disabled = !enabled;
        });

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

  loadPolicies() {
    get('/policies', {}, ({policies}) => {
      let el = this.elements['policies'];
      el.innerHTML = '';
      Object.keys(POLICY_SPEC).forEach((k) => {
        let args = [];
        let invalid = new Set();
        let spec = POLICY_SPEC[k];

        let fields = spec.args.map((arg) => {
          args.push({
            name: arg.name,
            value: arg.default
          });
          return `
            <div class="policy-item--field">
              <label>${arg.name}</label>
              <input class="policy-item--input" type="number" value="${arg.default}">
            </div>
            `;
        });
        let html = `<li class="policy-item">
          <div class="policy-item--name">${k}</div>
          <div class="policy-item--desc">${spec.desc}</div>
          <div class="policy-item--form">
            ${fields}
            <button disabled=true>Implement &gt;&gt;</button>
          </div>
        </li>`;

        // Setup editable inputs
        let child = htmlToElement(html);
        [...child.querySelectorAll('input')].forEach((input, i) => {
          let arg = spec.args[i];
          input.addEventListener('change', () => {
            let val;
            if (arg.type === 'int') {
              val = parseInt(input.value);
            } else if (arg.type == 'float') {
              val = parseFloat(input.value);
            }
            let isInvalid = isNaN(val)
              || (arg.min !== undefined && val < arg.min);

            if (isInvalid) {
              invalid.add(arg.name);
              input.style.background = '#ff8b8b';
            } else {
              input.value = val;
              args[i].value = val;
              invalid.delete(arg.name);
              input.style.background = '#eee';
            }
          });
        });
        child.querySelector('button').addEventListener('click', () => {
          if (invalid.size === 0) {
            let step = this.history.length;
            let current = this.policies[this.policies.length - 1];
            if (!current || current.step !== step) {
              current = {
                step: step,
                policies: []
              };
              this.policies.push(current);

              let html = `<div>
                <h3>Step ${step}</h3>
                <ul></ul>
              </div>`;
              let child = htmlToElement(html);
              this.elements['policyHistory'].prepend(child);
            }

            let policy = {
              name: k,
              args: args
            }
            current.policies.push(policy);

            let html = `<li>
              <h4>${k}</h4>
              <div class="policy--args">${args.map((a) => `${a.name}=${a.value}`).join('; ')}</div>
            </li>`;
            this.elements['policyHistory']
              .firstChild.querySelector('ul')
              .appendChild(htmlToElement(html));

            post('/policies', {
              name: k,
              args: args.map((a) => a.value),
            }, () => {});
          }
        });
        el.appendChild(child);
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
          let customVal;
          if (spec.type == 'int') {
            customVal = parseInt(inputEl.value);
          } else if (spec.type == 'float') {
            customVal = parseFloat(inputEl.value);
          }

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
