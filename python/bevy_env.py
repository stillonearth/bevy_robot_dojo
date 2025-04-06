import requests
import numpy as np
import json

API_STEP = "http://127.0.0.1:7878/step"
API_STATE = "http://127.0.0.1:7878/state"
API_RESET = "http://127.0.0.1:7878/reset"


class BevyRLEnv(object):

    def get_obs(self):
        state = requests.get(API_STATE).json()
        transforms = np.concatenate(
            [np.concatenate([t for t in state["transforms"]]), state["joint_angles"]]
        )
        return transforms

    def step_env(self, action):
        payload = json.dumps([{"action": json.dumps(action)}], indent=4)
        return requests.get(API_STEP, params={"payload": payload}).json()

    def step(self, action):
        obs = self.get_obs()
        step_data = self.step_env(action)[0]
        is_terminated = step_data["is_terminated"]
        reward = step_data["reward"]

        return obs, reward, is_terminated, False, {}, is_terminated

    def reset_env(self):
        requests.get(API_RESET)

    def reset(self):
        self.reset_env()

        return self.get_obs(), {}

    def render():
        None
