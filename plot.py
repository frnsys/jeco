import os
import json
import yaml
import matplotlib.pyplot as plt
from collections import defaultdict
from datetime import datetime

plt.style.use('ggplot')

def make_plots(output_dir):
    try:
        os.mkdir(os.path.join(output_dir, 'plots'))
    except FileExistsError:
        pass
    config = yaml.load(open(os.path.join(output_dir, 'config.yaml')))
    output = json.load(open(os.path.join(output_dir, 'output.json')))

    meta = output['meta']
    history = output['history']
    stats = defaultdict(list)

    for month in history:
        for k, v in month.items():
            stats[k].append(v)

    fnames = []

    # Group sample
    values = defaultdict(list)
    for month in stats['sample']:
        for s in month:
            values[s['id']].append(s['values'])

    plt.title('Agent Values')
    for id, vals in values.items():
        x, y = zip(*vals)
        plt.plot(list(x), list(y), label=id, marker='o')
    # plt.legend()
    plt.savefig(os.path.join(output_dir, 'plots/agent_values.png'))
    fnames.append('agent_values.png')

    with open(os.path.join(output_dir, 'plots/index.html'), 'w') as f:
        html = '''
            <html>
            <body style="font-family:monospace;">
                <h3>Generated on {dt}</h3>
                <div>
                    <div>{meta}</div>
                    <div>{config}</div>
                </div>
                <div>
                    {imgs}
                </div>
            </body>
            </html>
        '''.format(
            dt=datetime.now().isoformat(),
            config=json.dumps(config),
            meta=', '.join('{}: {}'.format(k, v) for k, v in meta.items()),
            imgs='\n'.join(['<img style="width:400px;" src="{}">'.format(fname) for fname in fnames]))
        f.write(html)


if __name__ == '__main__':
    make_plots('runs/latest')