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
  title: 'Subscribers',
  datasets: [{
    label: 'max',
    key: 'subscribers.max'
  }, {
    label: 'min',
    key: 'subscribers.min'
  }, {
    label: 'mean',
    key: 'subscribers.mean'
  }]
}, {
  title: 'Reach',
  datasets: [{
    label: 'max',
    key: 'reach.max'
  }, {
    label: 'min',
    key: 'reach.min'
  }, {
    label: 'mean',
    key: 'reach.mean'
  }]
}, {
  title: 'Budget',
  datasets: [{
    label: 'max',
    key: 'budget.max'
  }, {
    label: 'min',
    key: 'budget.min'
  }, {
    label: 'mean',
    key: 'budget.mean'
  }]
}, {
  title: 'Published',
  datasets: [{
    label: 'max',
    key: 'published.max'
  }, {
    label: 'min',
    key: 'published.min'
  }, {
    label: 'mean',
    key: 'published.mean'
  }]
}, {
  title: 'Publishability',
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

}];

const AGENT_SAMPLE = 15;
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
  key: 'publishers',
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
