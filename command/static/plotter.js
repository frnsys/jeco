Chart.defaults.scale.ticks.fontSize = 9;
Chart.defaults.scale.ticks.fontFamily = 'monospace';
Chart.defaults.global.tooltips.bodyFontFamily = 'monospace';
Chart.defaults.global.tooltips.cornerRadius = 0;

const POINT_RADIUS = 2;
const stage = document.getElementById('charts');
const space = document.getElementById('space');
const publishers = document.getElementById('publishers');


class Plotter {
  constructor(specs, scatters, colors) {
    this.colors = colors;
    this.charts = specs.map((s) => this.createChart(s));
    this.scatters = scatters.map((s) => this.createTimeScatterChart(s));
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
        tooltips: {
          callbacks: {
            label: (item, data) => {
              let label = data.datasets[item.datasetIndex].label || '';
              label = `${label}: ${Math.round(item.yLabel * 100) / 100}`;
              return label;
            }
          }
        },
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
        states.forEach((s) => {
          let value = valueFromKeyPath(s, spec.key);
          dataset.data.push(value);
        });
      });
      c.chart.update();
    });

    this.scatters.forEach((c) => {
      c.chart.data.datasets.forEach((dataset, i) => {
        states.forEach((s) => {
          let value = valueFromKeyPath(s, c.key);
          let item = value[i];
          if (item) {
            let pt = item[c.itemKey];
            dataset.data.push({x: pt[0], y: pt[1]});
          }
        });
        if (c.panel) {
          dataset.pointRadius = dataset.data.map(() => POINT_RADIUS/4);
          dataset.pointBackgroundColor = dataset.data.map((_, j) => this.color(i, j/dataset.data.length));
        } else {
          dataset.pointRadius = dataset.data.map((_, j) => j == (dataset.data.length-1) ? POINT_RADIUS : 0);
        }
      });
      c.chart.update();
      c.chart.sliderEl.value = 100;
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
    this.scatters.forEach((c) => {
      c.chart.data.datasets.forEach((dataset, i) => {
        dataset.data = [];
      });
      c.chart.update();
    });
    while (space.firstChild) {
      space.removeChild(space.firstChild);
    }
    while (publishers.firstChild) {
      publishers.removeChild(publishers.firstChild);
    }
  }

  color(i, alpha) {
    let [r, g, b] = this.colors[i%this.colors.length];
    alpha = alpha == undefined ? 1 : alpha;
    return `rgba(${r},${g},${b},${alpha})`
  }

  createTimeScatterChart(spec) {
    let chartEl = document.createElement('canvas');
    let chart = new Chart(chartEl, {
      type: 'scatter',
      data: {
        datasets: spec.datasets.map((d, i) => ({
          label: d.label,
          fill: false,
          showLine: spec.panel,
          borderWidth: 2,
          pointRadius: POINT_RADIUS,
          borderColor: this.color(i, 0.1),
          pointBackgroundColor: spec.panel ? [] : this.color(i),
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
    titleEl.innerText = spec.title;
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
        if (spec.panel) {
          d.pointRadius = d.data.map((_, i) => i > step ? 0 : POINT_RADIUS/4);
        } else {
          d.pointRadius = d.data.map((_, i) => i == step ? POINT_RADIUS : 0);
        }
      });
      chart.update();
    });

    chart.sliderEl = sliderEl;
    stage.appendChild(parentEl);
    return {
      key: spec.key,
      itemKey: spec.itemKey,
      panel: spec.panel,
      chart: chart
    };
  }

  createSpacePlot(state, gridSize) {
    const height = 200;
    const width = 200;
    const margin = {
      top: 20, right: 30,
      bottom: 30, left: 40
    };
    let agents = state.agents.map((a) => ({x: a.location[0], y: a.location[1]}));
    let publishers = Object.values(state.publishers.sample).map((p) => ({
      x: p.location[0] + (Math.random() - 0.5) * 0.2, // jitter
      y: p.location[1] + (Math.random() - 0.5) * 0.2,
      radius: p.radius
    }));

    let x = d3.scaleLinear()
        .domain(d3.extent(agents, d => d.x)).nice()
        .rangeRound([margin.left, width - margin.right])

    let y = d3.scaleLinear()
        .domain(d3.extent(agents, d => d.y)).nice()
        .rangeRound([height - margin.bottom, margin.top])

    let contours = d3.contourDensity()
        .x(d => x(d.x))
        .y(d => y(d.y))
        .size([width, height])
        .bandwidth(30)
        .thresholds(15)
      (agents)

    const colors = d3.scaleLinear()
      .domain([0, 0.1])
      .range(["#fff", "#888"]);

    const xScale = d3.scaleLinear()
        .domain([0, d3.max(publishers, function (d) { return d.x; })])
        .range([margin.left, width - margin.right]);
    const yScale = d3.scaleLinear()
        .domain([d3.max(publishers, function (d) { return d.y; }), 0])
        .range([margin.top, height - margin.bottom]);

    const r = 4;
    const mouseEnter = (d, i) => {
      svg.append('text')
        .attr('id', `t${i}`)
        .attr('font-size', '0.8em')
        .attr('x', xScale(d.x) + r)
        .attr('y', yScale(d.y) + r)
        .text(`Publisher ${i}`);
    };
    const mouseOut = (d, i) => {
      d3.select(`#t${i}`).remove();
    };

    const svg = d3.create('svg')
      .attr('viewBox', [0, 0, width, height]);

      svg.append('g')
          .attr('stroke-linejoin', 'round')
        .selectAll('path')
        .data(contours)
        .enter().append('path')
          .attr('fill', (d, i) => colors(d.value))
          .attr('stroke', (d, i) => colors(d.value))
          .attr('stroke-width', (d, i) => i % 5 ? 0.25 : 1)
          .attr('d', d3.geoPath());

      svg.append('g')
          .attr('stroke', 'none')
        .selectAll('circle')
        .data(publishers)
        .enter().append('circle')
          .attr('cx', d => x(d.x))
          .attr('cy', d => y(d.y))
          .attr('fill', (d, i) => this.color(i, 0.08))
          .attr('r', d => (d.radius+1)*(width/(gridSize*2)));

      svg.append('g')
          .attr('stroke', 'black')
        .selectAll('circle')
        .data(publishers)
        .enter().append('circle')
          .attr('fill', (d, i) => this.color(i))
          .attr('cx', d => x(d.x))
          .attr('cy', d => y(d.y))
          .attr('r', r)
          .on('mouseenter', mouseEnter)
          .on('mouseout', mouseOut);

    space.appendChild(svg.node());
  }

  listPublishers(state) {
    Object.keys(state.publishers.sample).forEach((id) => {
      let p = state.publishers.sample[id];
      let el = document.createElement('div');
      el.classList.add('publisher');
      let e = document.createElement('div');
      e.innerText = `Publisher ${id}`;
      e.style.fontWeight = 'bold';
      el.appendChild(e);

      Object.keys(p).forEach((k) => {
        let v = p[k];
        let e = document.createElement('div');
        e.innerText = `${k}: ${v}`;
        el.appendChild(e);
      });
      publishers.appendChild(el);
    });
  }
}
