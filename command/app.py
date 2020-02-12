import json
import redis
import config
from flask import Flask, jsonify, request, render_template

app = Flask(__name__)
redis = redis.Redis(**config.REDIS)

def send_command(cmd, data=None):
    redis.lpush('cmds', json.dumps({
        cmd: data
    }))


@app.route('/')
def index():
    return render_template('index.html')


@app.route('/status')
def status():
    """Query current sim status"""
    status = redis.get('status')
    if status: status = status.decode('utf8')
    return jsonify(status=status)


@app.route('/state/history')
def state_history():
    """Query state history range"""
    frm = request.args.get('from', 0)
    to = request.args.get('to', -1)
    print(request.args)
    history = [json.loads(r.decode('utf8')) for r in redis.lrange('state:history', int(frm), int(to))]
    return jsonify(history=history)


@app.route('/state/step')
def state_step():
    """Query current state step"""
    return jsonify(step=redis.get('state:step').decode('utf8'))


@app.route('/step', methods=['POST'])
def step():
    """Step the simulation"""
    data = request.get_json()
    send_command('Run', data['steps'])
    return jsonify(success=True)


@app.route('/reset', methods=['POST'])
def reset():
    """Reset the simulation"""
    send_command('Reset')
    return jsonify(success=True)


if __name__ == '__main__':
    app.run(host='0.0.0.0', port=8000, debug=True)