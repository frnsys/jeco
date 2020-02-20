const AGENT_SAMPLE = 15;
const PLATFORM_SAMPLE = 10;
const PUBLISHER_SAMPLE = 10;

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
}, {
  title: 'Value Shifts (sample)',
  datasets: [{
    label: 'max',
    key: 'value_shifts.max'
  }, {
    label: 'min',
    key: 'value_shifts.min'
  }, {
    label: 'mean',
    key: 'value_shifts.mean'
  }]
}, {
  title: 'p Producing',
  datasets: [{
    label: 'p',
    key: 'p_produced'
  }]
}, {
  title: 'Subscribers (Publishers)',
  datasets: [{
    label: 'max',
    key: 'publishers.stats.subscribers.max'
  }, {
    label: 'min',
    key: 'publishers.stats.subscribers.min'
  }, {
    label: 'mean',
    key: 'publishers.stats.subscribers.mean'
  }]
}, {
  title: 'Reach (Publishers)',
  datasets: [{
    label: 'max',
    key: 'publishers.stats.reach.max'
  }, {
    label: 'min',
    key: 'publishers.stats.reach.min'
  }, {
    label: 'mean',
    key: 'publishers.stats.reach.mean'
  }]
}, {
  title: 'Budget (Publishers)',
  datasets: [{
    label: 'max',
    key: 'publishers.stats.budget.max'
  }, {
    label: 'min',
    key: 'publishers.stats.budget.min'
  }, {
    label: 'mean',
    key: 'publishers.stats.budget.mean'
  }]
}, {
  title: 'Published (Publishers)',
  datasets: [{
    label: 'max',
    key: 'publishers.stats.published.max'
  }, {
    label: 'min',
    key: 'publishers.stats.published.min'
  }, {
    label: 'mean',
    key: 'publishers.stats.published.mean'
  }]
}, {
  title: 'Publishability (sample)',
  datasets: [{
    label: 'max',
    key: 'publishability.max'
  }, {
    label: 'min',
    key: 'publishability.min'
  }, {
    label: 'mean',
    key: 'publishability.mean'
  }]
}, {
  title: 'Resources (sample)',
  datasets: [{
    label: 'max',
    key: 'resources.max'
  }, {
    label: 'min',
    key: 'resources.min'
  }, {
    label: 'mean',
    key: 'resources.mean'
  }]
}, {
  title: 'Publisher Reach',
  datasets: [...Array(PUBLISHER_SAMPLE).keys()].map((i) => ({
    label: `${i}`,
    key: `publishers.sample.${i}.reach`
  }))
}, {
  title: 'Publisher Budget',
  datasets: [...Array(PUBLISHER_SAMPLE).keys()].map((i) => ({
    label: `${i}`,
    key: `publishers.sample.${i}.budget`
  }))
}, {
  title: 'Publisher Ads',
  datasets: [...Array(PUBLISHER_SAMPLE).keys()].map((i) => ({
    label: `${i}`,
    key: `publishers.sample.${i}.ads`
  }))
}, {
  title: 'Publisher Quality',
  datasets: [...Array(PUBLISHER_SAMPLE).keys()].map((i) => ({
    label: `${i}`,
    key: `publishers.sample.${i}.quality`
  }))
}, {
  title: 'Platform Users',
  datasets: [...Array(PLATFORM_SAMPLE).keys()].map((i) => ({
    label: `${i}`,
    key: `platforms.sample.${i}.users`
  }))
}, {
  title: 'Platform Data',
  datasets: [...Array(PLATFORM_SAMPLE).keys()].map((i) => ({
    label: `${i}`,
    key: `platforms.sample.${i}.data`
  }))
}];

const SCATTERS = [{
  title: 'Agent Values',
  key: 'agents',
  itemKey: 'values',
  panel: true,
  datasets: [...Array(AGENT_SAMPLE).keys()].map((i) => ({
    label: `Agent ${i}`,
  }))
}, {
  title: 'Most Popular Content Values',
  key: 'top_content',
  itemKey: 'values',
  panel: false,
  datasets: [...Array(10).keys()].map((i) => ({
    label: `Content ${i}`,
  }))
}, {
  title: 'Publisher Audience Understanding',
  key: 'publishers.audience',
  itemKey: 'values',
  panel: true,
  datasets: [...Array(10).keys()].map((i) => ({
    label: `Publisher ${i}`,
  }))
}]

const COLORS = [
  [  0, 153, 102],
  [230,   0,   0],
  [ 26, 102, 230],
  [102,  26, 230],
  [230,  77,   0],
  [254, 192,   7],
  [ 21, 211, 125],
];

const plotter = new Plotter(CHARTS, SCATTERS, COLORS);
const command = new Command({
  status: '#status',
  reset: '#reset button',
  configReset: '#reset--new-config',
  step: '#step button',
  stepInput: '#step input',
  config: '#config ul',
  policies: '#policy ul',
  policyHistory: '#policy-history div'
}, plotter);
