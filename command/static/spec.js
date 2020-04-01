const CONFIG_SPEC = {
  'POPULATION': {
    key: 'SIMULATION',
    type: 'int',
    desc: 'The number of agents to create. Higher numbers will run slower but can provide better results.'
  },
  'GRID_SIZE': {
    key: 'SIMULATION',
    type: 'int',
    desc: 'The size of the world. An n-by-n hex grid represents the world, where n is this value.'
  },
  'N_PUBLISHERS': {
    key: 'SIMULATION',
    type: 'int',
    desc: 'The number of publishers to create. Higher numbers will run slower but can provide better results.'
  },
  'N_PLATFORMS': {
    key: 'SIMULATION',
    type: 'int',
    desc: 'The number of platforms to create. Higher numbers will run slower but can provide better results.'
  },
  'CONTACT_RATE': {
    key: 'SIMULATION',
    type: 'float',
    desc: 'The base offline contact rate, i.e. probability that two agents share content in a step (without platforms).'
  },
  'ATTENTION_BUDGET': {
    key: 'SIMULATION.AGENT',
    type: 'float',
    desc: 'Attention budget for each agent. Limits how much content an agent can consume.'
  },
  'MAX_INFLUENCE': {
    key: 'SIMULATION',
    type: 'float',
    desc: 'Maximum amount a piece of content can influence a person\'s values.'
  },
  'GRAVITY_STRETCH': {
    key: 'SIMULATION',
    type: 'float',
    desc: 'Horizontal stretching of gravity function. Higher values mean weaker influence at greater distances.'
  },
  'DEFAULT_TRUST': {
    key: 'SIMULATION',
    type: 'float',
    desc: 'How much agents initially trust each other.'
  },
  'FOLLOW_TRUST': {
    key: 'SIMULATION',
    type: 'float',
    desc: 'If agent A\'s trust of agent B goes above this value, A follows B.'
  },
  'UNFOLLOW_TRUST': {
    key: 'SIMULATION',
    type: 'float',
    desc: 'If agent A\'s trust of agent B falls below this value, A unfollows B.'
  },
  'SUBSCRIBE_TRUST': {
    key: 'SIMULATION',
    type: 'float',
    desc: 'If agent A\'s trust of publisher B goes above this value, A subscribes to B.'
  },
  'UNSUBSCRIBE_TRUST': {
    key: 'SIMULATION',
    type: 'float',
    desc: 'If agent A\'s trust of publisher B falls below this value, A unsubscribes from B.'
  },
  'UNSUBSCRIBE_LAG': {
    key: 'SIMULATION',
    type: 'int',
    desc: 'Agents unsubscribe from publishers if they don\'t see content from them for this many steps.'
  },
  'CONTENT_SAMPLE_SIZE': {
    key: 'SIMULATION',
    type: 'int',
    desc: 'How much content a publisher looks at to understand its audience.'
  },
  'BASE_SIGNUP_RATE': {
    key: 'SIMULATION',
    type: 'float',
    desc: 'Base probability of signing up to a platform.',
  },
  'DATA_PER_CONSUME': {
    key: 'SIMULATION',
    type: 'float',
    desc: 'How much data is generated for a platform when a piece of content is consumed there.',
  },
  'MAX_PLATFORMS': {
    key: 'SIMULATION',
    type: 'int',
    desc: 'Max platforms an agent signs up for.',
  },
  'REVENUE_PER_AD': {
    key: 'SIMULATION',
    type: 'f32',
    desc: 'Revenue per ad view.',
  },
  'BASE_BUDGET': {
    key: 'SIMULATION.PUBLISHER',
    type: 'float',
    desc: 'Base budget for publishers. Determines how much content they can produce per step.',
  },
  'REVENUE_PER_SUBSCRIBER': {
    key: 'SIMULATION.PUBLISHER',
    type: 'float',
    desc: 'How much each subscriber adds to the publisher\'s budget.',
  },
  'BASE_CONVERSION_RATE': {
    key: 'SIMULATION',
    type: 'float',
    desc: 'Base conversion rate for ads.'
  },
  'MAX_CONVERSION_RATE': {
    key: 'SIMULATION',
    type: 'float',
    desc: 'Maximum conversion rate for ads.'
  },
  'COST_PER_QUALITY': {
    key: 'SIMULATION',
    type: 'float',
    desc: 'Cost to improve content quality by 1 point.'
  },
  'SEED': {
    key: null,
    type: 'int',
    desc: 'Use a consistent seed value to control for randomness across runs. You probably don\'t need to change this.'
  }
};

const PUBLISHER_SPEC = {
  'BASE_BUDGET': {
    type: 'float',
    desc: 'Base budget for the publisher.',
    default: 2000
  },
  'MOTIVE': {
    type: 'enum',
    choices: ['Profit', 'Influence', 'Civic'],
    desc: 'Publisher\'s motivation, which influences their decisions. One of "Profit", "Influence", or "Civic".',
    default: 'Civic'
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
  },
  'SubsidizeProduction': {
    desc: 'Increase the resources of publishers by the specified amount.',
    args: [{
      min: 0,
      type: 'float',
      name: 'amount',
      default: 100
    }]
  },
  'TaxAdvertising': {
    desc: 'Implement a tax of the specified amount on all advertising.',
    args: [{
      min: 0,
      type: 'float',
      name: 'tax',
      default: 0.1
    }]
  },
  'FoundPlatforms': {
    desc: 'Create new social media platforms.',
    args: [{
      min: 0,
      type: 'int',
      name: 'amount',
      default: 5
    }]
  },
  'MediaLiteracy': {
    desc: 'Improve media literacy.',
    args: [{
      min: 0,
      type: 'float',
      name: 'amount',
      default: 0.1
    }]
  },
  'Recession': {
    desc: 'Trigger economic recession.',
    args: [{
      min: 0,
      type: 'float',
      name: 'amount',
      default: 0.8
    }]
  }
};
