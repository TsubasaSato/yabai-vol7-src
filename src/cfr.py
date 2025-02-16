import numpy as np
from collections import defaultdict

# ベクトル演算を行う関数群
def add_assign_vec(lhs, rhs):
    np.add(lhs, rhs, out=lhs)

def sub_assign_vec(lhs, rhs):
    np.subtract(lhs, rhs, out=lhs)

def mul_vec(lhs, rhs):
    return np.multiply(lhs, rhs)

def mul_assign_scalar(vec, scalar):
    np.multiply(vec, scalar, out=vec)

def mul_assign_vec(lhs, rhs):
    np.multiply(lhs, rhs, out=lhs)

def div_assign_vec(lhs, rhs, default):
    with np.errstate(divide='ignore', invalid='ignore'):
        result = np.true_divide(lhs, rhs)
        result[np.isnan(result) | np.isinf(result)] = default
        lhs[:] = result

def nonneg_assign_vec(vec):
    np.maximum(vec, 0, out=vec)

# CFRアルゴリズムを管理するクラス
class CFRMinimizer:
    ALPHA = 1.5
    BETA = 0.0
    GAMMA = 2.0

    def __init__(self, game):
        self.game = game
        self.cum_regret = defaultdict(lambda: np.zeros((game.num_actions(), game.num_private_hands())))
        self.cum_strategy = defaultdict(lambda: np.zeros((game.num_actions(), game.num_private_hands())))
        self.alpha_t = 1.0
        self.beta_t = 1.0
        self.gamma_t = 1.0

    def compute(self, num_iterations):
        root = self.game.root()
        self.build_tree(root, self.cum_regret)
        self.build_tree(root, self.cum_strategy)
        ones = np.ones(self.game.num_private_hands())

        for t in range(1, num_iterations + 1):
            t_f64 = float(t)
            self.alpha_t = t_f64 ** self.ALPHA / (t_f64 ** self.ALPHA + 1.0)
            self.beta_t = t_f64 ** self.BETA / (t_f64 ** self.BETA + 1.0)
            self.gamma_t = (t_f64 + 1.0) ** self.GAMMA

            for player in range(2):
                self.cfr_recursive(root, player, ones, ones)

        return self.compute_average_strategy()

    def cfr_recursive(self, node, player, pi, pmi):
        if node.is_terminal():
            return self.game.evaluate(node, player, pmi)
        
        public_history = node.public_history()
        strategy = self.regret_matching(self.cum_regret[public_history])
        cfvalue = np.zeros(self.game.num_private_hands())

        if node.current_player() == player:
            cfvalue_action_vec = []
            for action in node.actions():
                pi_new = mul_vec(pi, strategy[action])
                cfvalue_action = self.cfr_recursive(node.play(action), player, pi_new, pmi)
                cfvalue_action_vec.append(cfvalue_action.copy())
                mul_assign_vec(cfvalue_action, strategy[action])
                add_assign_vec(cfvalue, cfvalue_action)

            for action in node.actions():
                cum_regret = self.cum_regret[public_history][action]
                cum_strategy = self.cum_strategy[public_history][action]
                cum_regret *= np.where(cum_regret >= 0, self.alpha_t, self.beta_t)
                add_assign_vec(cum_regret, cfvalue_action_vec[action])
                sub_assign_vec(cum_regret, cfvalue)
                mul_assign_scalar(strategy[action], self.gamma_t)
                mul_assign_vec(strategy[action], pi)
                add_assign_vec(cum_strategy, strategy[action])
        else:
            for action in node.actions():
                pmi_new = mul_vec(pmi, strategy[action])
                add_assign_vec(cfvalue, self.cfr_recursive(node.play(action), player, pi, pmi_new))

        return cfvalue

    def build_tree(self, node, tree):
        if node.is_terminal():
            return
        tree[node.public_history()] = np.zeros((node.num_actions(), self.game.num_private_hands()))
        for action in node.actions():
            self.build_tree(node.play(action), tree)

    def regret_matching(self, regrets):
        strategy = np.maximum(regrets, 0)
        denom = np.sum(strategy, axis=0, keepdims=True)
        return np.divide(strategy, denom, where=denom != 0, out=np.full_like(strategy, 1.0 / strategy.shape[0]))

    def compute_average_strategy(self):
        average_strategy = {k: v.copy() for k, v in self.cum_strategy.items()}
        for strategy in average_strategy.values():
            denom = np.sum(strategy, axis=0, keepdims=True)
            div_assign_vec(strategy, denom, 0.0)
        return average_strategy
