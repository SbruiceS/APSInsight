# frozen_string_literal: true

class ProjectPolicy < ApplicationPolicy
  def show?
    true
  end

  def create?
    true
  end
end
