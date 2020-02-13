Chart.defaults.scale.ticks.fontSize = 9;
Chart.defaults.scale.ticks.fontFamily = 'monospace';

const stage = document.getElementById('charts');

class Plotter {
  constructor(specs, colors) {
    this.colors = colors;
    this.charts = specs.map((s) => this.createChart(s));
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
  }

  reset() {
    this.charts.forEach((c) => {
      c.chart.data.labels = [];
      c.chart.data.datasets.forEach((dataset, i) => {
        dataset.data = [];
      });
      c.chart.update();
    });
  }
}
