# frozen_string_literal: true

require "rails_helper"

RSpec.describe Organization, type: :model do
  it "requires a unique slug" do
    Organization.create!(name: "A", slug: "a")
    dup = Organization.new(name: "B", slug: "a")
    expect(dup).not_to be_valid
  end
end

RSpec.describe Dataset, type: :model do
  it "belongs to project and organization" do
    expect(Dataset.reflect_on_association(:project).macro).to eq(:belongs_to)
    expect(Dataset.reflect_on_association(:organization).macro).to eq(:belongs_to)
  end
end

RSpec.describe Observation, type: :model do
  it "requires variable, value, unit, and observed_at" do
    obs = Observation.new
    expect(obs).not_to be_valid
    expect(obs.errors[:variable]).to be_present
    expect(obs.errors[:unit]).to be_present
  end
end
