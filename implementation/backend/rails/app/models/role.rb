# frozen_string_literal: true

class Role < ApplicationRecord
  has_many :memberships, dependent: :restrict_with_error

  validates :name, presence: true, uniqueness: true
  validates :key, presence: true, uniqueness: true, format: { with: /\A[a-z0-9_]+\z/ }
end
