use crate::interface::*;
use std::collections::HashMap;


// ベクトル演算を行う関数群
#[inline]
fn add_assign_vec(lhs: &mut Vec<f64>, rhs: &Vec<f64>) {
    // lhs の各要素に rhs の各要素を加算
    lhs.iter_mut().zip(rhs).for_each(|(l, r)| *l += *r);
}

#[inline]
fn sub_assign_vec(lhs: &mut Vec<f64>, rhs: &Vec<f64>) {
    // lhs の各要素から rhs の各要素を減算
    lhs.iter_mut().zip(rhs).for_each(|(l, r)| *l -= *r);
}

#[inline]
fn mul_vec(lhs: &Vec<f64>, rhs: &Vec<f64>) -> Vec<f64> {
    // lhs と rhs の要素ごとの積を計算し、新しいベクトルとして返す
    lhs.iter().zip(rhs).map(|(l, r)| l * r).collect()
}

#[inline]
fn mul_assign_scalar(vec: &mut Vec<f64>, scalar: f64) {
    // ベクトルの各要素をスカラー値で乗算
    vec.iter_mut().for_each(|el| *el *= scalar);
}

#[inline]
fn mul_assign_vec(lhs: &mut Vec<f64>, rhs: &Vec<f64>) {
    // lhs の各要素を rhs の各要素と乗算
    lhs.iter_mut().zip(rhs).for_each(|(l, r)| *l *= *r);
}

#[inline]
fn div_assign_vec(lhs: &mut Vec<f64>, rhs: &Vec<f64>, default: f64) {
    // lhs の各要素を rhs の各要素で割る。ただし、rhs の要素が 0.0 の場合は default を適用
    lhs.iter_mut().zip(rhs).for_each(|(l, r)| {
        *l = if *r == 0.0 { default } else { *l / *r };
    });
}

#[inline]
fn nonneg_assign_vec(vec: &mut Vec<f64>) {
    // 各要素を非負（0以上）にする
    vec.iter_mut().for_each(|el| *el = el.max(0.0));
}


/// CFRアルゴリズムを管理する構造体
pub struct CFRMinimizer<'a, T: Game> {
    /// ゲーム定義のインスタンス
    game: &'a T,

    /// リグレットの累積値
    cum_regret: HashMap<PublicHistory, Vec<Vec<f64>>>,

    /// 各時刻の戦略の和
    cum_strategy: HashMap<PublicHistory, Vec<Vec<f64>>>,

    /// Discounted CFR のパラメータ
    alpha_t: f64,

    /// Discounted CFR のパラメータ
    beta_t: f64,

    /// Discounted CFR のパラメータ
    gamma_t: f64,

    /// 終端ノード数
    // terminal_nodes : i32,
}

impl<'a, T: 'a + Game> CFRMinimizer<'a, T> {
    const ALPHA: f64 = 1.5;
    const BETA: f64 = 0.0;
    const GAMMA: f64 = 2.0;

    /// コンストラクタ
    pub fn new(game: &'a T) -> Self {
        Self {
            game,
            cum_regret: HashMap::new(),
            cum_strategy: HashMap::new(),
            alpha_t: 1.0,
            beta_t: 1.0,
            gamma_t: 1.0,
            // terminal_nodes: 0
        }
    }

    /// CFRアルゴリズムによる学習を行い、平均戦略を返す
    pub fn compute(&mut self, num_iterations: i32) -> HashMap<PublicHistory, Vec<Vec<f64>>> {
        // ゲームの初期履歴を取得
        let root = T::root();

        Self::build_tree(&root, &mut self.cum_regret);
        Self::build_tree(&root, &mut self.cum_strategy);
        
        // 到達確率を1で初期化
        // pmiは偶然手番による寄与が含まれない
        // すなわち、ハンドの組み合わせ確率がこの時点では考慮されない。
        // evaluateメソッド内で偶然手番による寄与を反映する。
        let ones = vec![1.0; T::num_private_hands()];

        // 自己対戦を繰り返す
        for t in 0..num_iterations {
            let t_f64 = t as f64;
            self.alpha_t = t_f64.powf(Self::ALPHA) / (t_f64.powf(Self::ALPHA) + 1.0);
            self.beta_t = t_f64.powf(Self::BETA) / (t_f64.powf(Self::BETA) + 1.0);
            self.gamma_t = (t_f64 + 1.0).powf(Self::GAMMA);

            // プレイヤー毎に処理を行う
            for player in 0..2 {
                self.cfr_recursive(&root, player, &ones, &ones);
            }
        }

        self.compute_average_strategy()
    }

    /// `player` の counterfactual value を再帰的に計算する
    fn cfr_recursive(
        &mut self,
        node: &T::Node,
        player: usize,
        pi: &Vec<f64>,
        pmi: &Vec<f64>,
    ) -> Vec<f64> {
        // 終端履歴なら単に counterfactual value を返す
        if node.is_terminal() {
            return self.game.evaluate(node, player, pmi);
        }

        // 現在のパブリックな履歴を取得
        let public_history = node.public_history();

        // 現時刻の戦略を regret-matching アルゴリズムによって求める
        let mut strategy = Self::regret_matching(&self.cum_regret[public_history]);

        // 返り値となる counterfactual value を0で初期化
        let mut cfvalue = vec![0.0; T::num_private_hands()];

        // 手番が `player` の場合
        if node.current_player() == player {
            let mut cfvalue_action_vec = Vec::with_capacity(node.num_actions());

            // 各アクションに対する counterfactual value を計算する
            for action in node.actions() {
                let pi = mul_vec(&pi, &strategy[action]);
                // アクションにおけるcfv（利得*到達確率）
                let mut cfvalue_action:Vec<f64>= self.cfr_recursive(&node.play(action), player, &pi, pmi);
                cfvalue_action_vec.push(cfvalue_action.clone());
                // アクションにおけるcfv（利得*到達確率）* アクションの各ハンドにおける最適戦略確率
                mul_assign_vec(&mut cfvalue_action, &strategy[action]);
                add_assign_vec(&mut cfvalue, &cfvalue_action);
            }

            // リグレットの累積値と戦略の和を更新
            for action in node.actions() {
                let cum_regret: &mut Vec<f64> =
                    &mut self.cum_regret.get_mut(public_history).unwrap()[action];
                let cum_strategy: &mut Vec<f64> =
                    &mut self.cum_strategy.get_mut(public_history).unwrap()[action];

                cum_regret.iter_mut().for_each(|el| {
                    *el *= if *el >= 0.0 {
                        self.alpha_t
                    } else {
                        self.beta_t
                    }
                });

                // DCRFにおける、T+1回目のri(I,a)=vi(σI→a, h)-vi(σ, h)をリグレットの累積値に加算する
                  // cfvalue_action_vec[action]：アクションにおけるcfv（利得*到達確率）
                add_assign_vec(cum_regret, &cfvalue_action_vec[action]);
                  // cfvalue：各アクションにおけるcfv（利得*到達確率）* 各アクションの最適戦略確率　における全アクションの和
                sub_assign_vec(cum_regret, &cfvalue);

                //DCFRにおける、ナッシュ均衡に収束するためのCF到達確率で重みづけたTまでの戦略の和
                //※まだ平均戦略ではないことに注意(compute_average_strategyメソッドでTまでのCF到達確率の和で割り算する)
                mul_assign_scalar(&mut strategy[action], self.gamma_t);
                mul_assign_vec(&mut strategy[action], &pi);
                add_assign_vec(cum_strategy, &strategy[action]);
            }
        }
        // 手番が `player` でない場合
        else {
            for action in node.actions() {
                let pmi = mul_vec(&pmi, &strategy[action]);
                add_assign_vec(
                    &mut cfvalue,
                    &self.cfr_recursive(&node.play(action), player, pi, &pmi),
                );
            }
        }

        cfvalue
    }

    /// ゲーム木を構築する
    fn build_tree(node: &T::Node, tree: &mut HashMap<PublicHistory, Vec<Vec<f64>>>) {
        if node.is_terminal() {
            return;
        }

        tree.insert(
            node.public_history().clone(),
            vec![vec![0.0; T::num_private_hands()]; node.num_actions()],
        );

        for action in node.actions() {
            Self::build_tree(&node.play(action), tree);
        }
    }

    /// ゲーム木を構築する
    // fn build_tree(&mut self, node: &T::Node) {
    //     if node.is_terminal() {
    //         self.terminal_nodes +=1;
    //         return;
    //     }

    //     self.cum_regret.insert(
    //         node.public_history().clone(),
    //         vec![vec![0.0; T::num_private_hands()]; node.num_actions()],
    //     );

    //     for action in node.actions() {
    //         self.build_tree(&node.play(action));
    //     }
    // }

    /// 終端ノード数を返す
    fn get_terminal_nodes(&self)-> i32{
        self.terminal_nodes
    }

    /// regret-matching アルゴリズム
    fn regret_matching(regrets: &Vec<Vec<f64>>) -> Vec<Vec<f64>> {
        let num_actions = regrets.len();
        let num_private_hands = T::num_private_hands();
        let mut strategy = regrets.clone();

        let mut denom = vec![0.0; num_private_hands];
        strategy.iter_mut().for_each(|strategy_action| {
            nonneg_assign_vec(strategy_action);
            add_assign_vec(&mut denom, strategy_action);
        });

        strategy.iter_mut().for_each(|strategy_action| {
            div_assign_vec(strategy_action, &denom, 1.0 / num_actions as f64);
        });

        strategy
    }

    /// フィールド `cum_strategy` を参照して平均戦略を返す
    /// この時点では`cum_strategy`はナッシュ均衡に収束するためのCF到達確率で重みづけたTまでの戦略の和(P23の分子)
    /// 
    /// 平均戦略はとあるノードにおけるアクションをそのノードで選択できる全アクションの戦略で割った割合
    /// とあるノードでベットとコールの２アクションが可能であれば、ベットの戦略/ベットとコール　が平均戦略
    fn compute_average_strategy(&self) -> HashMap<PublicHistory, Vec<Vec<f64>>> {
        let num_private_hands = T::num_private_hands();
        let mut average_strategy: HashMap<PublicHistory, Vec<Vec<f64>>> = self.cum_strategy.clone();

        // 全ノードにおける戦略を確認している
        for strategy in average_strategy.values_mut() {
            let mut denom = vec![0.0; num_private_hands];
            // とあるノードにおける戦略の合計を作成する（例：ベット、コールの２アクションが可能なノード）
            strategy.iter().for_each(|strategy_action| {
                add_assign_vec(&mut denom, &strategy_action);
            });

            // とあるノードにおいてアクション/アクションの戦略和　を計算して、average_strategyを更新
            strategy.iter_mut().for_each(|strategy_action| {
                div_assign_vec(strategy_action, &denom, 0.0);
            });
        }

        average_strategy
    }
}
