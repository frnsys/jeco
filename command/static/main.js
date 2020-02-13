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
  [  0, 153, 102],
  [230,   0,   0],
  [ 26, 102, 230],
  [102,  26, 230],
  [230,  77,   0],
  [254, 192,   7],
  [ 21, 211, 125],
];

const plotter = new Plotter(CHARTS, COLORS);
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
