import os
import json
import yaml
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.collections as mcoll
import matplotlib.colors as mcolors
from collections import defaultdict
from datetime import datetime

plt.style.use('ggplot')

figsize=(10, 8)

cmaps = []
colors = [
    (0.0, 0.6, 0.4),
    (0.9, 0.0, 0.0),
    (0.1, 0.4, 0.9),
    (0.4, 0.1, 0.9),
    (0.9, 0.3, 0.0),
]

for r, g, b in colors:
    name = '{}_{}_{}'.format(r, g, b)
    cdict = {
        'red': [
            (0., r, r),
            (1., r, r)
        ],
        'green': [
            (0., g, g),
            (1., g, g)
        ],
        'blue':  [
            (0., b, b),
            (1., b, b)
        ],
        'alpha': [
            (0., 0.0, 0.0),
            (1., 1.0, 1.0)
        ]
    }
    cmap = mcolors.LinearSegmentedColormap(name, cdict)
    cmaps.append(cmap)

def colorline(x, y, cmap, norm=plt.Normalize(0.0, 1.0), linewidth=2):
    # Default colors equally spaced on [0,1]:
    z = np.linspace(0.0, 1.0, len(x))
    z = np.asarray(z)

    segments = make_segments(x, y)
    lc = mcoll.LineCollection(segments, array=z, cmap=cmap,
                              norm=norm,
                              linewidth=linewidth)

    ax = plt.gca()
    ax.add_collection(lc)

    return lc


def make_segments(x, y):
    """
    Create list of line segments from x and y coordinates, in the correct format
    for LineCollection: an array of the form numlines x (points per line) x 2 (x
    and y) array
    """

    points = np.array([x, y]).T.reshape(-1, 1, 2)
    segments = np.concatenate([points[:-1], points[1:]], axis=1)
    return segments


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

    plt.figure(figsize=figsize)
    plt.title('Agent Values')
    plt.ylim([-1, 1])
    plt.xlim([-1, 1])
    for i, (id, vals) in enumerate(values.items()):
        x, y = zip(*vals)

        cmap = cmaps[i%len(cmaps)]
        colorline(x, y, cmap=cmap)
    # plt.legend()
    plt.savefig(os.path.join(output_dir, 'plots/agent_values.png'))
    fnames.append('agent_values.png')

    for k in ['to_share', 'p_produced']:
        fname = '{}.png'.format(k)
        plt.figure(figsize=figsize)
        plt.title(k)
        vals = stats[k]
        plt.plot(range(len(vals)), vals)
        plt.savefig(os.path.join(output_dir, 'plots/{}'.format(fname)))
        fnames.append(fname)

    for k in ['shares', 'followers']:
        fname = '{}.png'.format(k)
        plt.figure(figsize=figsize)
        plt.title(k)
        grouped = defaultdict(list)
        for month in stats[k]:
            for k_, v_ in month.items():
                grouped[k_].append(v_)
        for k_, vals in grouped.items():
            plt.plot(range(len(vals)), vals, label=k_)
        plt.legend()
        plt.savefig(os.path.join(output_dir, 'plots/{}'.format(fname)))
        fnames.append(fname)

    for k in ['share_dist', 'follower_dist']:
        fname = '{}.png'.format(k)
        plt.figure(figsize=figsize)
        plt.title('mean {} 0-dropped'.format(k))
        bins = defaultdict(int)
        for month in stats[k]:
            for bin, count in month.items():
                bins[int(bin)] += count
        x = []
        mn = min(bins.keys())
        mx = max(bins.keys())
        for i in range(mn, mx):
            v = bins.get(i, 0)/len(stats[k])
            x.append(v)
        plt.xticks(list(range(len(x)))[1:])
        plt.bar(list(range(len(x)))[1:], x[1:])
        plt.savefig(os.path.join(output_dir, 'plots/{}'.format(fname)))
        fnames.append(fname)

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
            imgs='\n'.join(['<img style="width:600px;" src="{}">'.format(fname) for fname in fnames]))
        f.write(html)


if __name__ == '__main__':
    make_plots('runs/latest')