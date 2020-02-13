Chart.defaults.scale.ticks.fontSize = 9;
Chart.defaults.scale.ticks.fontFamily = 'monospace';
Chart.defaults.global.tooltips.bodyFontFamily = 'monospace';
Chart.defaults.global.tooltips.cornerRadius = 0;

const POINT_RADIUS = 2;
const AGENT_SAMPLE = 15;

const stage = document.getElementById('charts');


class Plotter {
  constructor(specs, colors) {
    this.colors = colors;
    this.charts = specs.map((s) => this.createChart(s));
    this.trajChart = this.createTrajectoriesChart();
  }

  createChart(spec) {
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
          backgroundColor: this.color(i),
          borderColor: this.color(i),
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

    let titleEl = document.createElement('div');
    titleEl.innerText = spec.title;
    titleEl.className = 'chart-title';

    let parentEl = document.createElement('div');
    parentEl.className = 'chart';
    parentEl.appendChild(titleEl);
    parentEl.appendChild(chartEl);
    stage.appendChild(parentEl);

    return {
      datasets: spec.datasets,
      chart: chart
    };
  }

  append(states) {
    this.charts.forEach((c) => {
      states.forEach((s) => {
        c.chart.data.labels.push(s.step);
      });
      c.chart.data.datasets.forEach((dataset, i) => {
        let spec = c.datasets[i];
        let keyPath = spec.key.split('.');
        states.forEach((s) => {
          let value = keyPath.slice(1)
            .reduce((acc, k) => acc[k], s[keyPath[0]]);
          dataset.data.push(value);
        });
      });
      c.chart.update();
    });

    this.trajChart.data.datasets.forEach((dataset, i) => {
      states.forEach((s) => {
        let pt = s.sample[i].values;
        dataset.data.push({x: pt[0], y: pt[1]});
      });
      dataset.pointBackgroundColor = dataset.data.map((_, j) => this.color(i, (j/dataset.data.length)**2));
      dataset.pointRadius = dataset.data.map(() => POINT_RADIUS);
    });
    this.trajChart.update();
    this.trajChart.sliderEl.value = 100;
  }

  reset() {
    this.charts.forEach((c) => {
      c.chart.data.labels = [];
      c.chart.data.datasets.forEach((dataset, i) => {
        dataset.data = [];
      });
      c.chart.update();
    });
    this.trajChart.data.datasets.forEach((dataset, i) => {
      dataset.data = [];
    });
    this.trajChart.update();
  }

  color(i, alpha) {
    let [r, g, b] = this.colors[i%this.colors.length];
    alpha = alpha || 1;
    return `rgba(${r},${g},${b},${alpha})`
  }

  createTrajectoriesChart() {
    let chartEl = document.createElement('canvas');
    let chart = new Chart(chartEl, {
      type: 'scatter',
      data: {
        datasets: [...Array(AGENT_SAMPLE).keys()].map((i) => ({
          label: `Agent ${i}`,
          fill: false,
          showLine: true,
          borderWidth: 2,
          pointRadius: POINT_RADIUS,
          borderColor: this.color(i, 0.1),
          pointBackgroundColor: [],
          data: []
        }))
      },
      options: {
        tooltips: {
          callbacks: {
            label: (item, data) => {
              let label = data.datasets[item.datasetIndex].label || '';
              label = `${label}: ${Math.round(item.xLabel * 100) / 100}, ${Math.round(item.yLabel * 100) / 100}`;
              return label;
            }
          }
        },
        animation: {
          duration: 0
        },
        legend: {
          display: false,
        },
        scales: {
          yAxes: [{
            ticks: {
              min: -1,
              max: 1
            },
            display: true
          }],
          xAxes: [{
            ticks: {
              min: -1,
              max: 1
            },
            display: true
          }]
        }
      }
    });

    let titleEl = document.createElement('div');
    titleEl.innerText = 'Agent Values';
    titleEl.className = 'chart-title';

    let parentEl = document.createElement('div');
    parentEl.className = 'chart';
    parentEl.appendChild(titleEl);
    parentEl.appendChild(chartEl);

    let sliderEl = document.createElement('input');
    sliderEl.type = 'range';
    sliderEl.min = 0;
    sliderEl.max = 100;
    sliderEl.value = 100;
    parentEl.appendChild(sliderEl);
    sliderEl.addEventListener('input', () => {
      let steps = chart.data.datasets[0].data.length;
      let step = Math.floor(steps * (sliderEl.value/100));
      chart.data.datasets.forEach((d) => {
        d.pointRadius = d.data.map((_, i) => i > step ? 0 : POINT_RADIUS);
      });
      chart.update();
    });

    chart.sliderEl = sliderEl;
    stage.appendChild(parentEl);
    return chart;
  }
}
